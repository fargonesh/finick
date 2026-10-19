//! A dedicated ext-session-lock-v1 client backed by per-output Wayland SHM surfaces.
//!
//! This is intentionally separate from Winit windows and layer-shell surfaces.
//! The compositor must acknowledge the lock before the call can report success.
//! The initial API renders frames but does not dispatch keyboard or pointer
//! input or mount a Freya `App`/`Runner`; unlock must come from another thread
//! through the atomic control flag, and the caller supplies the Skia draw callback.
//! After requesting a lock, local setup errors do not cancel that request:
//! the client keeps dispatching until `locked` or `finished`. If dispatch itself
//! cannot recover before either event, this function stays alive rather than
//! dropping a pending session lock.

use std::{
    sync::{
        Arc,
        atomic::{
            AtomicBool,
            Ordering,
        },
    },
    time::Duration,
};

use smithay_client_toolkit::{
    compositor::{
        CompositorHandler,
        CompositorState,
    },
    delegate_compositor,
    delegate_output,
    delegate_registry,
    delegate_session_lock,
    delegate_shm,
    output::{
        OutputHandler,
        OutputState,
    },
    reexports::{
        calloop::EventLoop,
        calloop_wayland_source::WaylandSource,
        client::{
            Connection,
            Proxy,
            QueueHandle,
            globals::registry_queue_init,
            protocol::{
                wl_buffer,
                wl_output,
                wl_shm,
                wl_surface,
            },
        },
    },
    registry::{
        ProvidesRegistryState,
        RegistryState,
    },
    registry_handlers,
    session_lock::{
        SessionLock,
        SessionLockHandler,
        SessionLockState,
        SessionLockSurface,
        SessionLockSurfaceConfigure,
    },
    shm::{
        Shm,
        ShmHandler,
    },
};

pub use crate::wayland_native::NativeWaylandError;
use crate::wayland_native::{
    NativeRenderCallback,
    attach_shm_frame,
};

/// Run an ext-session-lock-v1 client.
///
/// One lock surface is created for every output, including outputs added while
/// the client is running. The function waits for `locked` before treating the
/// session as locked. A compositor `finished` event or missing protocol is an
/// error, never a fallback to a normal or layer-shell window.
///
/// Set `unlock_requested` to request an orderly unlock. Requests made before
/// the compositor confirms the lock are deferred until the `locked` event.
/// Every configured output is rendered through an opaque SHM buffer. If the
/// callback fails, that surface receives a black buffer and the lock remains
/// held until explicitly requested to unlock.
///
/// ```no_run
/// use std::sync::{
///     Arc,
///     atomic::AtomicBool,
/// };
///
/// use freya_winit::wayland_session_lock::run_session_lock;
///
/// let unlock = Arc::new(AtomicBool::new(false));
/// run_session_lock(unlock, |_, surface| {
///     surface.canvas().clear(freya_core::prelude::Color::BLACK);
///     Ok(())
/// })?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn run_session_lock(
    unlock_requested: Arc<AtomicBool>,
    render: impl FnMut((u32, u32), &mut freya_engine::prelude::Surface) -> Result<(), String> + 'static,
) -> Result<(), NativeWaylandError> {
    let connection = Connection::connect_to_env()
        .map_err(|error| NativeWaylandError(format!("Wayland connection failed: {error}")))?;
    let (globals, event_queue) = registry_queue_init(&connection)
        .map_err(|error| NativeWaylandError(format!("Wayland registry failed: {error}")))?;
    let qh = event_queue.handle();
    let compositor = CompositorState::bind(&globals, &qh)
        .map_err(|error| NativeWaylandError(format!("wl_compositor unavailable: {error}")))?;
    let shm = Shm::bind(&globals, &qh)
        .map_err(|error| NativeWaylandError(format!("wl_shm unavailable: {error}")))?;
    if !shm.formats().contains(&wl_shm::Format::Argb8888) {
        return Err(NativeWaylandError("ARGB8888 SHM format unavailable".into()));
    }
    let output_state = OutputState::new(&globals, &qh);
    let outputs: Vec<_> = output_state.outputs().collect();
    if outputs.is_empty() {
        return Err(NativeWaylandError(
            "Session lock requires at least one Wayland output".into(),
        ));
    }
    let mut event_loop: EventLoop<LockClient> = EventLoop::try_new()
        .map_err(|error| NativeWaylandError(format!("Wayland event loop failed: {error}")))?;
    WaylandSource::new(connection.clone(), event_queue)
        .insert(event_loop.handle())
        .map_err(|error| NativeWaylandError(format!("Wayland event source failed: {error}")))?;

    let session_lock_state = SessionLockState::new(&globals, &qh);
    let session_lock = session_lock_state
        .lock(&qh)
        .map_err(|error| NativeWaylandError(format!("ext-session-lock-v1 unavailable: {error}")))?;

    let mut client = LockClient {
        registry_state: RegistryState::new(&globals),
        output_state,
        compositor_state: compositor,
        shm,
        _session_lock_state: session_lock_state,
        session_lock: Some(session_lock),
        lock_surfaces: Vec::new(),
        render: Box::new(render),
        phase: LockPhase::Requested,
        render_error: None,
        protocol_error: None,
    };

    for output in outputs {
        client.add_output(&qh, output);
    }

    loop {
        if let Err(error) = event_loop.dispatch(Duration::from_millis(50), &mut client) {
            client.protocol_error = Some(format!("Wayland dispatch failed: {error}"));
            if client.phase == LockPhase::Requested {
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
        }

        match client.phase {
            LockPhase::Finished => break,
            LockPhase::Locked if client.protocol_error.is_some() => break,
            LockPhase::Locked if unlock_requested.load(Ordering::Acquire) => break,
            LockPhase::Requested | LockPhase::Locked => {}
            LockPhase::Unlocked => break,
        }
    }

    let phase_at_shutdown = client.phase;
    if let Some(lock) = client.session_lock.take() {
        lock.unlock();
    }
    let unlock_sync = flush_and_sync(&connection, "Session unlock");
    client.lock_surfaces.clear();
    let surfaces_sync = flush_and_sync(&connection, "Lock surfaces");
    if phase_at_shutdown == LockPhase::Locked {
        client.phase = LockPhase::Unlocked;
    }
    let teardown_error = unlock_sync.and(surfaces_sync).err();

    if phase_at_shutdown == LockPhase::Finished {
        let message = teardown_error.map_or_else(
            || "Compositor denied or finished the session lock".to_string(),
            |error| format!("Compositor denied or finished the session lock; {error}"),
        );
        return Err(NativeWaylandError(message));
    }
    if let Some(error) = teardown_error {
        return Err(error);
    }
    if let Some(error) = client.protocol_error.take() {
        return Err(NativeWaylandError(error));
    }
    if let Some(error) = client.render_error.take() {
        return Err(NativeWaylandError(format!(
            "Session remained locked with black fallback surfaces: {error}"
        )));
    }
    Ok(())
}

fn flush_and_sync(connection: &Connection, operation: &str) -> Result<(), NativeWaylandError> {
    let flush = connection
        .flush()
        .map_err(|error| format!("{operation} flush failed: {error}"));
    let sync = connection
        .roundtrip()
        .map(|_| ())
        .map_err(|error| format!("{operation} sync failed: {error}"));
    flush.and(sync).map_err(NativeWaylandError)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LockPhase {
    Requested,
    Locked,
    Finished,
    Unlocked,
}

impl LockPhase {
    fn on_locked(self) -> Self {
        match self {
            Self::Requested => Self::Locked,
            phase => phase,
        }
    }

    fn on_finished(self) -> Self {
        match self {
            Self::Unlocked => Self::Unlocked,
            _ => Self::Finished,
        }
    }
}

struct LockClient {
    registry_state: RegistryState,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm: Shm,
    _session_lock_state: SessionLockState,
    session_lock: Option<SessionLock>,
    lock_surfaces: Vec<(wl_output::WlOutput, SessionLockSurface)>,
    render: NativeRenderCallback,
    phase: LockPhase,
    render_error: Option<String>,
    protocol_error: Option<String>,
}

impl LockClient {
    fn add_output(&mut self, qh: &QueueHandle<Self>, output: wl_output::WlOutput) {
        if self.phase == LockPhase::Finished {
            return;
        }
        if self
            .lock_surfaces
            .iter()
            .any(|(known, _)| known.id() == output.id())
        {
            return;
        }
        let Some(lock) = self.session_lock.as_ref() else {
            self.protocol_error = Some("New output arrived after lock teardown".into());
            return;
        };
        let surface = self.compositor_state.create_surface(qh);
        let lock_surface = lock.create_lock_surface(surface, &output, qh);
        lock_surface.wl_surface().commit();
        self.lock_surfaces.push((output, lock_surface));
    }
}

impl SessionLockHandler for LockClient {
    fn locked(&mut self, _: &Connection, _: &QueueHandle<Self>, _: SessionLock) {
        self.phase = self.phase.on_locked();
    }

    fn finished(&mut self, _: &Connection, _: &QueueHandle<Self>, _: SessionLock) {
        self.phase = self.phase.on_finished();
    }

    fn configure(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        lock_surface: SessionLockSurface,
        configure: SessionLockSurfaceConfigure,
        _: u32,
    ) {
        if self.phase == LockPhase::Finished {
            return;
        }
        match attach_shm_frame(
            &self.shm,
            qh,
            lock_surface.wl_surface(),
            configure.new_size.0,
            configure.new_size.1,
            &mut self.render,
        ) {
            Ok(None) => {}
            Ok(Some(error)) => self.render_error = Some(error),
            Err(error) => self.protocol_error = Some(error.to_string()),
        }
    }
}

impl CompositorHandler for LockClient {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: i32,
    ) {
    }
    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}
    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for LockClient {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, qh: &QueueHandle<Self>, output: wl_output::WlOutput) {
        self.add_output(qh, output);
    }
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        self.lock_surfaces
            .retain(|(known, _)| known.id() != output.id());
    }
}

impl ProvidesRegistryState for LockClient {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState,];
}

impl ShmHandler for LockClient {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_compositor!(LockClient);
delegate_output!(LockClient);
delegate_registry!(LockClient);
delegate_session_lock!(LockClient);
delegate_shm!(LockClient);
smithay_client_toolkit::reexports::client::delegate_noop!(LockClient: ignore wl_buffer::WlBuffer);

#[cfg(test)]
mod tests {
    use super::LockPhase;

    #[test]
    fn only_confirmed_locks_enter_the_locked_phase() {
        assert_eq!(LockPhase::Requested.on_locked(), LockPhase::Locked);
        assert_eq!(LockPhase::Finished.on_locked(), LockPhase::Finished);
    }

    #[test]
    fn a_finished_lock_is_not_an_unlock_success() {
        assert_ne!(LockPhase::Finished, LockPhase::Unlocked);
        assert_eq!(LockPhase::Locked.on_finished(), LockPhase::Finished);
        assert_eq!(LockPhase::Unlocked.on_finished(), LockPhase::Unlocked);
    }
}
