//! finick-lock (P1): ext-session-lock-v1 client with password + fingerprint unlock.
//!
//! Flow: bind manager -> `lock()` -> one lock surface per `wl_output` ->
//! render frame on every surface -> wait for `locked` -> password dots and
//! fingerprint ring; correct password (via `system::verify_password`) or
//! fingerprint match (via fprintd) -> `unlock_and_destroy()` + `sync` before exit.
//!
//! Exit codes: 0 = unlocked cleanly; 42 = lock denied/superseded (`finished`)
//! or manager missing, caller should fall back to the Freya locker; 1 = error.
//!
//! Hard rule enforced here: **every** exit path while locked sends
//! `unlock_and_destroy` first and waits for the sync callback. No
//! `process::exit` mid-lock. A panic hook attempts unlock as last resort.
//! Never run on a live session for testing: spare TTY + minimal Hyprland.

mod finger;
mod render;

use {
    calloop::{
        EventLoop,
        channel::{self, Channel},
        timer::{TimeoutAction, Timer},
    },
    calloop_wayland_source::WaylandSource,
    finger::FingerEvent,
    std::{
        collections::HashMap,
        os::unix::io::{AsFd, OwnedFd},
        sync::{
            Mutex, OnceLock,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    },
    wayland_client::{
        Connection, Dispatch, Proxy, QueueHandle, WEnum, delegate_noop,
        protocol::{
            wl_buffer, wl_callback, wl_compositor, wl_display, wl_keyboard, wl_output, wl_registry, wl_seat, wl_shm,
            wl_shm_pool, wl_surface,
        },
    },
    wayland_protocols::ext::session_lock::v1::client::{
        ext_session_lock_manager_v1::{self, ExtSessionLockManagerV1},
        ext_session_lock_surface_v1::{self, ExtSessionLockSurfaceV1},
        ext_session_lock_v1::{self, ExtSessionLockV1},
    },
};

/// Exit code telling the caller to fall back to the Freya locker.
const EXIT_FALLBACK: i32 = 42;
/// configure with 0 size means "no requirement yet"; still commit something.
const FALLBACK_SIZE: (i32, i32) = (1920, 1080);
/// XKB keysyms (fixed by the XKB spec).
const KEY_RETURN: u32 = 0xff0d;
const KEY_BACKSPACE: u32 = 0xff08;
const KEY_ESCAPE: u32 = 0xff1b;

/// Events from helper threads (password check, fingerprint) to the main loop.
enum BgEvent {
    Auth(bool),
    Finger(FingerEvent),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum FingerStatus {
    Unavailable,
    Waiting,
    Match,
    NoMatch,
}

/// Minimal xkbcommon wrapper over the dlopened lib (no link-time dep).
/// All pointers live on the Wayland thread only.
struct Xkb {
    keymap: *mut xkbcommon_dl::xkb_keymap,
    state: *mut xkbcommon_dl::xkb_state,
}

// ponytail: raw xkb pointers confined to one thread; never shared.
unsafe impl Send for Xkb {}

impl Xkb {
    fn load(keymap_fd: OwnedFd, size: u32) -> Option<Self> {
        use std::{ffi::CString, io::Read};
        let xkb = xkbcommon_dl::xkbcommon_option()?;
        let mut file = std::fs::File::from(keymap_fd);
        let mut buf = vec![0u8; size as usize];
        file.read_exact(&mut buf).ok()?;
        buf.push(0);
        let cstr = CString::new(buf).ok()?;
        unsafe {
            let ctx = (xkb.xkb_context_new)(xkbcommon_dl::xkb_context_flags::XKB_CONTEXT_NO_FLAGS);
            if ctx.is_null() {
                return None;
            }
            let keymap = (xkb.xkb_keymap_new_from_string)(
                ctx,
                cstr.as_ptr(),
                xkbcommon_dl::xkb_keymap_format::XKB_KEYMAP_FORMAT_TEXT_V1,
                xkbcommon_dl::xkb_keymap_compile_flags::XKB_KEYMAP_COMPILE_NO_FLAGS,
            );
            (xkb.xkb_context_unref)(ctx);
            if keymap.is_null() {
                return None;
            }
            let state = (xkb.xkb_state_new)(keymap);
            if state.is_null() {
                (xkb.xkb_keymap_unref)(keymap);
                return None;
            }
            Some(Self { keymap, state })
        }
    }

    fn press(&mut self, evdev_key: u32) -> Option<KeyPress> {
        let xkb = xkbcommon_dl::xkbcommon_option()?;
        let code = evdev_key + 8;
        unsafe {
            (xkb.xkb_state_update_key)(self.state, code, xkbcommon_dl::xkb_key_direction::XKB_KEY_DOWN);
            let sym = (xkb.xkb_state_key_get_one_sym)(self.state, code);
            if sym == KEY_RETURN {
                return Some(KeyPress::Submit);
            }
            if sym == KEY_BACKSPACE {
                return Some(KeyPress::Backspace);
            }
            if sym == KEY_ESCAPE {
                return Some(KeyPress::Clear);
            }
            let mut buf = [0 as std::os::raw::c_char; 32];
            let n = (xkb.xkb_state_key_get_utf8)(self.state, code, buf.as_mut_ptr(), buf.len());
            if n > 0 {
                let bytes = &buf[..n as usize].iter().map(|c| *c as u8).collect::<Vec<_>>();
                if let Ok(s) = std::str::from_utf8(bytes) {
                    if !s.is_empty() {
                        return Some(KeyPress::Text(s.to_string()));
                    }
                }
            }
            None
        }
    }

    fn release(&mut self, evdev_key: u32) {
        if let Some(xkb) = xkbcommon_dl::xkbcommon_option() {
            unsafe {
                (xkb.xkb_state_update_key)(self.state, evdev_key + 8, xkbcommon_dl::xkb_key_direction::XKB_KEY_UP);
            }
        }
    }

    fn modifiers(&mut self, depressed: u32, latched: u32, locked: u32, group: u32) {
        if let Some(xkb) = xkbcommon_dl::xkbcommon_option() {
            unsafe {
                (xkb.xkb_state_update_mask)(self.state, depressed, latched, locked, 0, 0, group);
            }
        }
    }
}

impl Drop for Xkb {
    fn drop(&mut self) {
        if let Some(xkb) = xkbcommon_dl::xkbcommon_option() {
            unsafe {
                (xkb.xkb_state_unref)(self.state);
                (xkb.xkb_keymap_unref)(self.keymap);
            }
        }
    }
}

enum KeyPress {
    Text(String),
    Backspace,
    Clear,
    Submit,
}

// Panic-hook unlock slot: filled on lock(), armed on `locked`, disarmed after unlock.
static PANIC_SLOT: OnceLock<Mutex<Option<(Connection, ExtSessionLockV1)>>> = OnceLock::new();
static PANIC_ARMED: AtomicBool = AtomicBool::new(false);

fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if PANIC_ARMED.swap(false, Ordering::SeqCst) {
            if let Some(mut guard) = PANIC_SLOT.get().and_then(|m| m.lock().ok()) {
                if let Some((conn, lock)) = guard.take() {
                    drop(guard);
                    // Best effort: the alternative is a permanently locked session.
                    lock.unlock_and_destroy();
                    let _ = conn.flush();
                    let _ = conn.roundtrip();
                }
            }
        }
        prev(info);
    }));
}

fn disarm_panic_hook() {
    PANIC_ARMED.store(false, Ordering::SeqCst);
    if let Some(mut guard) = PANIC_SLOT.get().and_then(|m| m.lock().ok()) {
        guard.take();
    }
}

struct Output {
    output: wl_output::WlOutput,
    surface: Option<wl_surface::WlSurface>,
    lock_surface: Option<ExtSessionLockSurfaceV1>,
    buffer: Option<wl_buffer::WlBuffer>,
    /// Logical size from configure; pixels are size × scale.
    w: i32,
    h: i32,
    scale: i32,
    bg_cache: Option<(i32, i32, Vec<u32>)>,
}

struct Locker {
    conn: Connection,
    display: wl_display::WlDisplay,
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    manager: Option<ExtSessionLockManagerV1>,
    lock: Option<ExtSessionLockV1>,
    outputs: HashMap<u32, Output>,
    seat: Option<wl_seat::WlSeat>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    xkb: Option<Xkb>,
    bg_tx: Option<channel::Sender<BgEvent>>, // user data for keyboard Dispatch
    username: String,
    password: String,
    auth_pending: bool,
    pw_failed: bool,
    finger: FingerStatus,
    theme: render::Theme,
    locked: bool,
    waiting_sync: bool,
    exit_code: Option<i32>,
}

impl Locker {
    /// Create the lock object and a lock surface for every known output.
    fn request_lock(&mut self, qh: &QueueHandle<Self>) {
        let manager = match self.manager.clone() {
            Some(m) => m,
            None => {
                self.exit_code = Some(EXIT_FALLBACK);
                return;
            }
        };
        let lock = manager.lock(qh, ());
        let names: Vec<u32> = self.outputs.keys().copied().collect();
        for name in names {
            self.make_lock_surface(qh, &lock, name);
        }
        if let Some(mut guard) = PANIC_SLOT.get().and_then(|m| m.lock().ok()) {
            guard.replace((self.conn.clone(), lock.clone()));
        }
        self.lock = Some(lock);
        manager.destroy();
    }

    fn make_lock_surface(&mut self, qh: &QueueHandle<Self>, lock: &ExtSessionLockV1, name: u32) {
        let (Some(compositor), Some(out)) = (self.compositor.clone(), self.outputs.get_mut(&name)) else {
            return;
        };
        if out.lock_surface.is_some() {
            return; // duplicate_output is a protocol error; one surface per output.
        }
        let surface = compositor.create_surface(qh, ());
        surface.set_buffer_scale(out.scale);
        let lock_surface = lock.get_lock_surface(&surface, &out.output, qh, name);
        out.surface = Some(surface);
        out.lock_surface = Some(lock_surface);
    }

    fn on_configure(&mut self, qh: &QueueHandle<Self>, name: u32, serial: u32, w: i32, h: i32) {
        let (w, h) = if w > 0 && h > 0 { (w, h) } else { FALLBACK_SIZE };
        // Ack immediately; every later commit is then covered.
        if let Some(out) = self.outputs.get(&name).and_then(|o| o.lock_surface.clone()) {
            out.ack_configure(serial);
        }
        if let Some(out) = self.outputs.get_mut(&name) {
            out.w = w;
            out.h = h;
        }
        self.draw_frame(qh, name);
    }

    fn draw_frame(&mut self, qh: &QueueHandle<Self>, name: u32) {
        // Resolve pixel size + hint before borrowing the output mutably.
        let (w, h, scale) = match self.outputs.get(&name) {
            Some(o) => (o.w, o.h, o.scale.max(1)),
            None => return,
        };
        if w <= 0 || h <= 0 {
            return;
        }
        let (pw, ph) = (w * scale, h * scale);
        let hint = self.hint_text();
        let overlays = render::Overlays {
            dots: self.password.chars().count(),
            dots_fail: self.pw_failed,
            finger: self.finger,
            clock: render::clock_text(self.theme.use_24h),
            hint: hint.as_deref(),
            font: self.theme.font.clone(),
        };
        let Some(out) = self.outputs.get_mut(&name) else { return };
        let (Some(shm), Some(surface)) = (self.shm.clone(), out.surface.clone()) else {
            return;
        };
        if out.bg_cache.as_ref().is_none_or(|(cw, ch, _)| *cw != pw || *ch != ph) {
            out.bg_cache = Some((pw, ph, render::background_pixels(&self.theme.bg, pw, ph)));
        }
        let mut pixels = out.bg_cache.as_ref().map(|(_, _, px)| px.clone()).unwrap_or_default();
        render::draw_overlays(&mut pixels, pw, ph, &overlays);
        let stride = pw * 4;
        let size = (stride * ph) as usize;
        let mut file = match tempfile::tempfile() {
            Ok(f) => f,
            Err(e) => {
                eprintln!("finick-lock: shm tempfile failed: {e}");
                return;
            }
        };
        if file.set_len(size as u64).is_err() {
            return;
        }
        {
            use std::io::Write;
            // ponytail: byte view of one buffer, no bytemuck dep for this.
            let bytes: &[u8] = unsafe { std::slice::from_raw_parts(pixels.as_ptr() as *const u8, size) };
            if file.write_all(bytes).is_err() {
                return;
            }
        }
        let pool = shm.create_pool(file.as_fd(), size as i32, qh, ());
        let buffer = pool.create_buffer(0, pw, ph, stride, wl_shm::Format::Argb8888, qh, ());
        pool.destroy();
        surface.attach(Some(&buffer), 0, 0);
        surface.commit();
        out.buffer = Some(buffer);
    }

    fn hint_text(&self) -> Option<String> {
        if self.auth_pending {
            return Some("Checking…".to_string());
        }
        match self.finger {
            FingerStatus::Waiting => Some("Touch the sensor or type your password".to_string()),
            FingerStatus::NoMatch => Some("No match — try again".to_string()),
            FingerStatus::Unavailable => Some("Type your password to unlock".to_string()),
            FingerStatus::Match => None,
        }
    }

    fn redraw_all(&mut self, qh: &QueueHandle<Self>) {
        let names: Vec<u32> = self.outputs.keys().copied().collect();
        for name in names {
            self.draw_frame(qh, name);
        }
    }

    fn on_key(&mut self, qh: &QueueHandle<Self>, bg_tx: &channel::Sender<BgEvent>, evdev_key: u32, pressed: bool) {
        if !pressed {
            if let Some(xkb) = self.xkb.as_mut() {
                xkb.release(evdev_key);
            }
            return;
        }
        let press = match self.xkb.as_mut() {
            Some(xkb) => xkb.press(evdev_key),
            None => {
                eprintln!("finick-lock: no keymap, keyboard unusable (fingerprint only)");
                return;
            }
        };
        match press {
            None => {}
            Some(KeyPress::Backspace) => {
                self.pw_failed = false;
                self.password.pop();
                self.redraw_all(qh);
            }
            Some(KeyPress::Clear) => {
                self.pw_failed = false;
                self.password.clear();
                self.redraw_all(qh);
            }
            Some(KeyPress::Text(s)) => {
                self.pw_failed = false;
                if self.password.len() < 256 {
                    self.password.push_str(&s);
                }
                self.redraw_all(qh);
            }
            Some(KeyPress::Submit) => self.submit_password(bg_tx),
        }
    }

    fn submit_password(&mut self, bg_tx: &channel::Sender<BgEvent>) {
        if self.auth_pending || self.password.is_empty() || !self.locked {
            return;
        }
        self.auth_pending = true;
        let (user, pw, tx) = (self.username.clone(), std::mem::take(&mut self.password), bg_tx.clone());
        std::thread::spawn(move || {
            // Blocking auth (PAM stack, then legacy chain) must never stall the Wayland loop.
            let ok = system::authenticate_user(&user, &pw);
            let _ = tx.send(BgEvent::Auth(ok));
        });
    }

    fn on_bg(&mut self, qh: &QueueHandle<Self>, ev: BgEvent) {
        match ev {
            BgEvent::Auth(ok) => {
                self.auth_pending = false;
                if !self.locked || self.exit_code.is_some() {
                    return;
                }
                if ok {
                    eprintln!("finick-lock: password correct, unlocking");
                    self.unlock(qh);
                } else {
                    // Wrong password: stay locked, clear, flash red dots.
                    self.pw_failed = true;
                    self.password.clear();
                    self.redraw_all(qh);
                }
            }
            BgEvent::Finger(fev) => {
                match fev {
                    FingerEvent::Ready => self.finger = FingerStatus::Waiting,
                    FingerEvent::Match => self.finger = FingerStatus::Match,
                    FingerEvent::NoMatch => self.finger = FingerStatus::NoMatch,
                    FingerEvent::Retry => {
                        if self.finger == FingerStatus::NoMatch {
                            self.finger = FingerStatus::Waiting;
                        }
                    }
                    FingerEvent::Gone => self.finger = FingerStatus::Unavailable,
                }
                if fev == FingerEvent::Match {
                    if self.locked {
                        eprintln!("finick-lock: fingerprint match, unlocking");
                        self.unlock(qh);
                        return;
                    }
                    // Match before `locked`: ignore, sensor stays armed.
                    self.finger = FingerStatus::Waiting;
                }
                self.redraw_all(qh);
            }
        }
    }

    /// Unlock path: valid only after `locked`. Waits for the sync callback.
    fn unlock(&mut self, qh: &QueueHandle<Self>) {
        if self.waiting_sync || self.exit_code.is_some() {
            return;
        }
        if !self.locked {
            // Never locked: nothing to release; destroy the pending object.
            if let Some(lock) = self.lock.take() {
                lock.destroy();
            }
            disarm_panic_hook();
            self.exit_code = Some(1);
            return;
        }
        if let Some(lock) = self.lock.take() {
            lock.unlock_and_destroy();
            // Async protocol: the sync roundtrip guarantees the compositor
            // processes the unlock before we exit.
            self.display.sync(qh, ());
            self.waiting_sync = true;
        } else {
            disarm_panic_hook();
            self.exit_code = Some(1);
        }
    }

    /// `finished` (denied or superseded): fall back to the Freya locker.
    fn fallback(&mut self, qh: &QueueHandle<Self>) {
        if self.exit_code.is_some() {
            return;
        }
        if self.locked {
            // Superseded after lock: release our (dead) object, keep 42 so the
            // caller runs the fallback path.
            if let Some(lock) = self.lock.take() {
                lock.unlock_and_destroy();
                self.display.sync(qh, ());
                self.waiting_sync = true;
                self.exit_code = Some(EXIT_FALLBACK);
                return;
            }
        } else if let Some(lock) = self.lock.take() {
            lock.destroy();
        }
        disarm_panic_hook();
        let _ = self.conn.flush();
        self.exit_code = Some(EXIT_FALLBACK);
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for Locker {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global { name, interface, version } => {
                if interface == wl_compositor::WlCompositor::interface().name && state.compositor.is_none() {
                    state.compositor =
                        Some(registry.bind::<wl_compositor::WlCompositor, _, _>(name, version.min(6), qh, ()));
                } else if interface == wl_shm::WlShm::interface().name && state.shm.is_none() {
                    state.shm = Some(registry.bind::<wl_shm::WlShm, _, _>(name, 1, qh, ()));
                } else if interface == ExtSessionLockManagerV1::interface().name && state.manager.is_none() {
                    state.manager = Some(registry.bind::<ExtSessionLockManagerV1, _, _>(name, 1, qh, ()));
                } else if interface == wl_seat::WlSeat::interface().name && state.seat.is_none() {
                    state.seat = Some(registry.bind::<wl_seat::WlSeat, _, _>(name, version.min(9), qh, ()));
                } else if interface == wl_output::WlOutput::interface().name && !state.outputs.contains_key(&name) {
                    let output = registry.bind::<wl_output::WlOutput, _, _>(name, version.min(4), qh, ());
                    state.outputs.insert(name, Output {
                        output,
                        surface: None,
                        lock_surface: None,
                        buffer: None,
                        w: 0,
                        h: 0,
                        scale: 1,
                        bg_cache: None,
                    });
                    // Hotplug while locked: cover the new output immediately.
                    if let Some(lock) = state.lock.clone() {
                        state.make_lock_surface(qh, &lock, name);
                    }
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                if let Some(mut out) = state.outputs.remove(&name) {
                    if let Some(ls) = out.lock_surface.take() {
                        ls.destroy();
                    }
                    if let Some(s) = out.surface.take() {
                        s.destroy();
                    }
                    out.output.release();
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtSessionLockV1, ()> for Locker {
    fn event(
        state: &mut Self,
        _: &ExtSessionLockV1,
        event: ext_session_lock_v1::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            ext_session_lock_v1::Event::Locked => {
                if state.exit_code.is_none() {
                    state.locked = true;
                    PANIC_ARMED.store(true, Ordering::SeqCst);
                    eprintln!("finick-lock: session locked");
                }
            }
            ext_session_lock_v1::Event::Finished => {
                eprintln!("finick-lock: lock finished by compositor, falling back");
                state.fallback(qh);
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtSessionLockSurfaceV1, u32> for Locker {
    fn event(
        state: &mut Self,
        _: &ExtSessionLockSurfaceV1,
        event: ext_session_lock_surface_v1::Event,
        output_name: &u32,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let ext_session_lock_surface_v1::Event::Configure { serial, width, height } = event {
            state.on_configure(qh, *output_name, serial, width as i32, height as i32);
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for Locker {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities { capabilities } = event {
            if let WEnum::Value(caps) = capabilities {
                if caps.contains(wl_seat::Capability::Keyboard) && state.keyboard.is_none() {
                    if let Some(tx) = state.bg_tx.clone() {
                        state.keyboard = Some(seat.get_keyboard(qh, tx));
                    }
                }
            }
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, channel::Sender<BgEvent>> for Locker {
    fn event(
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        bg_tx: &channel::Sender<BgEvent>,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Keymap { format, fd, size } => {
                if let WEnum::Value(f) = format {
                    if f == wl_keyboard::KeymapFormat::XkbV1 {
                        match Xkb::load(fd, size) {
                            Some(xkb) => state.xkb = Some(xkb),
                            None => eprintln!("finick-lock: keymap load failed, fingerprint only"),
                        }
                    }
                }
            }
            wl_keyboard::Event::Key { key, state: key_state, .. } => {
                if let WEnum::Value(s) = key_state {
                    state.on_key(qh, bg_tx, key, s == wl_keyboard::KeyState::Pressed);
                }
            }
            wl_keyboard::Event::Modifiers { mods_depressed, mods_latched, mods_locked, group, .. } => {
                if let Some(xkb) = state.xkb.as_mut() {
                    xkb.modifiers(mods_depressed, mods_latched, mods_locked, group);
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_callback::WlCallback, ()> for Locker {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            if state.waiting_sync {
                state.waiting_sync = false;
                disarm_panic_hook();
                // Preserve a stashed fallback code (superseded case).
                if state.exit_code.is_none() {
                    state.exit_code = Some(0);
                }
            }
        }
    }
}

impl Dispatch<wl_output::WlOutput, ()> for Locker {
    fn event(
        state: &mut Self,
        output: &wl_output::WlOutput,
        event: wl_output::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Scale { factor } = event {
            let scale = factor.max(1);
            let name = state.outputs.iter().find(|(_, o)| o.output == *output).map(|(n, _)| *n);
            if let Some(name) = name {
                let changed = match state.outputs.get_mut(&name) {
                    Some(o) => {
                        let changed = o.scale != scale;
                        o.scale = scale;
                        if let Some(s) = o.surface.clone() {
                            s.set_buffer_scale(scale);
                        }
                        changed
                    }
                    None => false,
                };
                if changed {
                    state.draw_frame(qh, name);
                }
            }
        }
    }
}

delegate_noop!(Locker: ignore wl_compositor::WlCompositor);
delegate_noop!(Locker: ignore wl_surface::WlSurface);
delegate_noop!(Locker: ignore wl_shm::WlShm);
delegate_noop!(Locker: ignore wl_shm_pool::WlShmPool);
delegate_noop!(Locker: ignore wl_buffer::WlBuffer);
delegate_noop!(Locker: ignore wl_display::WlDisplay);
delegate_noop!(Locker: ignore ext_session_lock_manager_v1::ExtSessionLockManagerV1);

fn usage() -> ! {
    eprintln!("usage: finick-lock [--timeout <secs>]");
    eprintln!("  Locks the session via ext-session-lock-v1 until SIGINT/SIGTERM or --timeout.");
    std::process::exit(2);
}

fn main() {
    let mut timeout: Option<Duration> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--timeout" => {
                let secs: u64 = args.next().unwrap_or_else(|| usage()).parse().unwrap_or_else(|_| usage());
                timeout = Some(Duration::from_secs(secs));
            }
            "--help" | "-h" => usage(),
            _ => usage(),
        }
    }

    PANIC_SLOT.set(Mutex::new(None)).ok();
    install_panic_hook();

    let conn = Connection::connect_to_env().unwrap_or_else(|e| {
        eprintln!("finick-lock: no wayland connection: {e}");
        std::process::exit(1);
    });
    let display = conn.display();
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    display.get_registry(&qh, ());

    let mut locker = Locker {
        conn: conn.clone(),
        display,
        compositor: None,
        shm: None,
        manager: None,
        lock: None,
        outputs: HashMap::new(),
        seat: None,
        keyboard: None,
        xkb: None,
        bg_tx: None,
        username: system::current_username(),
        password: String::new(),
        auth_pending: false,
        pw_failed: false,
        finger: FingerStatus::Unavailable,
        theme: render::load_theme(),
        locked: false,
        waiting_sync: false,
        exit_code: None,
    };

    // Initial registry roundtrip: learn outputs/compositor/shm/manager.
    if event_queue.roundtrip(&mut locker).is_err() {
        eprintln!("finick-lock: initial roundtrip failed");
        std::process::exit(1);
    }
    if locker.compositor.is_none() || locker.shm.is_none() {
        eprintln!("finick-lock: compositor or shm missing");
        std::process::exit(1);
    }
    if locker.outputs.is_empty() {
        eprintln!("finick-lock: no outputs advertised");
        std::process::exit(1);
    }
    if locker.manager.is_none() {
        eprintln!("finick-lock: ext_session_lock_manager_v1 not advertised (fallback needed)");
        std::process::exit(EXIT_FALLBACK);
    }

    locker.request_lock(&qh);
    // Roundtrip so configure events are acked and first frames committed
    // before entering the loop (compositor may wait for them pre-`locked`).
    if event_queue.roundtrip(&mut locker).is_err() {
        eprintln!("finick-lock: lock roundtrip failed");
        std::process::exit(1);
    }
    if let Some(code) = locker.exit_code {
        // Denied immediately (finished during roundtrip).
        std::process::exit(code);
    }

    let mut event_loop: EventLoop<Locker> = EventLoop::try_new().unwrap_or_else(|e| {
        eprintln!("finick-lock: event loop failed: {e}");
        std::process::exit(1);
    });
    WaylandSource::new(conn, event_queue).insert(event_loop.handle()).unwrap_or_else(|e| {
        eprintln!("finick-lock: wayland source failed: {e:?}");
        std::process::exit(1);
    });

    // Helper-thread results (password check, fingerprint) back to the loop.
    // Keyboard Dispatch carries a clone of the sender as user data.
    let (bg_tx, bg_rx): (channel::Sender<BgEvent>, Channel<BgEvent>) = channel::channel();
    locker.bg_tx = Some(bg_tx.clone());
    event_loop
        .handle()
        .insert_source(bg_rx, {
            let qh = qh.clone();
            move |event, _, locker: &mut Locker| {
                if let channel::Event::Msg(ev) = event {
                    locker.on_bg(&qh, ev);
                }
            }
        })
        .unwrap();
    finger::spawn(bg_tx, locker.username.clone());

    // SIGINT/SIGTERM -> unlock. Every other exit path goes through unlock() too.
    event_loop
        .handle()
        .insert_source(
            calloop::signals::Signals::new(&[calloop::signals::Signal::SIGINT, calloop::signals::Signal::SIGTERM]).unwrap(),
            {
                let qh = qh.clone();
                move |_, _, locker: &mut Locker| {
                    eprintln!("finick-lock: signal received, unlocking");
                    locker.unlock(&qh);
                }
            },
        )
        .unwrap();

    if let Some(d) = timeout {
        event_loop
            .handle()
            .insert_source(Timer::from_duration(d), {
                let qh = qh.clone();
                move |_, _, locker: &mut Locker| {
                    eprintln!("finick-lock: timeout reached, unlocking");
                    locker.unlock(&qh);
                    TimeoutAction::Drop
                }
            })
            .unwrap();
    }

    // Clock refresh: one shot per minute boundary, then re-armed.
    event_loop
        .handle()
        .insert_source(Timer::from_duration(render::seconds_to_next_minute()), {
            let qh = qh.clone();
            move |_, _, locker: &mut Locker| {
                locker.redraw_all(&qh);
                TimeoutAction::ToDuration(render::seconds_to_next_minute())
            }
        })
        .unwrap();

    loop {
        if let Err(e) = event_loop.dispatch(None, &mut locker) {
            // Connection broken: unlock is best-effort only from here.
            eprintln!("finick-lock: dispatch error: {e}");
            locker.unlock(&qh);
            let _ = locker.conn.flush();
            std::process::exit(locker.exit_code.unwrap_or(1));
        }
        if let Some(code) = locker.exit_code {
            // waiting_sync guarantees the unlock landed before we return.
            if !locker.waiting_sync {
                std::process::exit(code);
            }
        }
    }
}
