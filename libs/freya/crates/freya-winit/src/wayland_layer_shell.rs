//! A dedicated SCTK wlr-layer-shell client backed by Wayland SHM.
//!
//! This path does not create or alter a Winit window. It runs a native Wayland
//! event loop and renders each compositor-configured frame through Skia.
//! The callback runs at initial configure and later size changes; this first
//! version does not yet forward keyboard or pointer events, or mount a Freya
//! `App`/`Runner`; the caller supplies the Skia draw callback.

use std::time::Duration;

pub use smithay_client_toolkit::shell::wlr_layer::{
    Anchor,
    KeyboardInteractivity,
    Layer,
};
use smithay_client_toolkit::{
    compositor::{
        CompositorHandler,
        CompositorState,
    },
    delegate_compositor,
    delegate_layer,
    delegate_output,
    delegate_registry,
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
    shell::{
        WaylandSurface,
        wlr_layer::{
            LayerShell,
            LayerShellHandler,
            LayerSurface,
            LayerSurfaceConfigure,
        },
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

/// Options for a standalone wlr-layer-shell surface.
#[derive(Debug, Clone)]
pub struct LayerShellConfig {
    pub width: u32,
    pub height: u32,
    pub namespace: String,
    pub layer: Layer,
    pub anchor: Anchor,
    pub keyboard_interactivity: KeyboardInteractivity,
}

impl LayerShellConfig {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            namespace: "freya".into(),
            layer: Layer::Top,
            anchor: Anchor::empty(),
            keyboard_interactivity: KeyboardInteractivity::None,
        }
    }
}

/// Run a standalone wlr-layer-shell client until the compositor closes it.
///
/// Rendering happens only after a configure event, using a compositor-sized
/// SHM buffer. The callback receives a Skia surface shared with Freya's regular
/// software backend. Winit windows and behavior are unaffected.
///
/// ```no_run
/// use freya_winit::wayland_layer_shell::{
///     LayerShellConfig,
///     run_layer_shell,
/// };
///
/// run_layer_shell(LayerShellConfig::new(640, 80), |_, surface| {
///     surface.canvas().clear(freya_core::prelude::Color::BLACK);
///     Ok(())
/// })?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn run_layer_shell(
    config: LayerShellConfig,
    render: impl FnMut((u32, u32), &mut freya_engine::prelude::Surface) -> Result<(), String> + 'static,
) -> Result<(), NativeWaylandError> {
    if config.width == 0 || config.height == 0 {
        return Err(NativeWaylandError(
            "Layer-shell requested size must be non-zero".into(),
        ));
    }
    let connection = Connection::connect_to_env()
        .map_err(|error| NativeWaylandError(format!("Wayland connection failed: {error}")))?;
    let (globals, event_queue) = registry_queue_init(&connection)
        .map_err(|error| NativeWaylandError(format!("Wayland registry failed: {error}")))?;
    let qh = event_queue.handle();
    let compositor = CompositorState::bind(&globals, &qh)
        .map_err(|error| NativeWaylandError(format!("wl_compositor unavailable: {error}")))?;
    let layer_shell = LayerShell::bind(&globals, &qh)
        .map_err(|error| NativeWaylandError(format!("wlr-layer-shell unavailable: {error}")))?;
    let shm = Shm::bind(&globals, &qh)
        .map_err(|error| NativeWaylandError(format!("wl_shm unavailable: {error}")))?;
    if !shm.formats().contains(&wl_shm::Format::Argb8888) {
        return Err(NativeWaylandError("ARGB8888 SHM format unavailable".into()));
    }
    let surface = compositor.create_surface(&qh);
    let layer = layer_shell.create_layer_surface(
        &qh,
        surface,
        config.layer,
        Some(config.namespace.clone()),
        None,
    );
    layer.set_anchor(config.anchor);
    layer.set_keyboard_interactivity(config.keyboard_interactivity);
    layer.set_size(config.width, config.height);
    layer.commit();

    let mut event_loop: EventLoop<LayerClient> = EventLoop::try_new()
        .map_err(|error| NativeWaylandError(format!("Wayland event loop failed: {error}")))?;
    let mut state = LayerClient {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        shm,
        render: Box::new(render),
        requested_size: (config.width, config.height),
        _layer: layer,
        error: None,
        closed: false,
    };
    WaylandSource::new(connection.clone(), event_queue)
        .insert(event_loop.handle())
        .map_err(|error| NativeWaylandError(format!("Wayland event source failed: {error}")))?;

    while !state.closed {
        if let Err(error) = event_loop.dispatch(Duration::from_millis(100), &mut state) {
            return Err(NativeWaylandError(format!(
                "Wayland dispatch failed: {error}"
            )));
        }
    }
    connection
        .flush()
        .map_err(|error| NativeWaylandError(format!("Wayland flush failed: {error}")))?;
    match state.error.take() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

struct LayerClient {
    registry_state: RegistryState,
    output_state: OutputState,
    shm: Shm,
    render: NativeRenderCallback,
    requested_size: (u32, u32),
    _layer: LayerSurface,
    error: Option<NativeWaylandError>,
    closed: bool,
}

impl CompositorHandler for LayerClient {
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

impl LayerShellHandler for LayerClient {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.closed = true;
    }

    fn configure(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        let (width, height) = configure.new_size;
        let size = (
            if width == 0 {
                self.requested_size.0
            } else {
                width
            },
            if height == 0 {
                self.requested_size.1
            } else {
                height
            },
        );
        match attach_shm_frame(
            &self.shm,
            qh,
            layer.wl_surface(),
            size.0,
            size.1,
            &mut self.render,
        ) {
            Ok(None) => {}
            Ok(Some(error)) => {
                self.error = Some(NativeWaylandError(error));
                self.closed = true;
            }
            Err(error) => {
                self.error = Some(error);
                self.closed = true;
            }
        }
    }
}

impl OutputHandler for LayerClient {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl ProvidesRegistryState for LayerClient {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState,];
}

impl ShmHandler for LayerClient {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_compositor!(LayerClient);
delegate_layer!(LayerClient);
delegate_output!(LayerClient);
delegate_registry!(LayerClient);
delegate_shm!(LayerClient);
smithay_client_toolkit::reexports::client::delegate_noop!(LayerClient: ignore wl_buffer::WlBuffer);
