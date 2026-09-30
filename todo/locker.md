# Finick locker: session-lock scope

## Background

The window-layer locker (Freya window + Hyprland float/pin/audit rules +
finickd respawn supervisor) is bypassable by design: any shell access defeats
it (`SUPER+Return` still works while locked, `watch pkill locker` + removing
the lockfile wins every race). It is a deterrent, not a lock.

The correct fix is `ext-session-lock-v1` (what hyprlock uses): the compositor
itself blocks input and blanks outputs. Nothing in the current stack can do
that — Freya/winit windows are regular surfaces and receive no input (and are
not shown) while a session lock is held.

## Verdict

Build a dedicated session-lock client. The Rust bindings already exist
in-tree (`wayland_protocols::ext::session_lock`, `staging` feature) and the
protocol surface is tiny (~330 lines of XML, 4 requests, 3 events).

## Protocol flow (per ext-session-lock-v1.xml)

1. Bind `ext_session_lock_manager_v1` → `lock()` → `ext_session_lock_v1`.
2. For every `wl_output`: `get_lock_surface()` — exactly one surface per
   output (duplicates are a protocol error); cover newly advertised outputs.
3. Render + commit a frame on every surface → compositor sends `locked`
   (guaranteed: no unlocked pixels visible anywhere once this fires).
4. Password wrong → stay locked (local only). Password correct →
   `unlock_and_destroy()` **then `wl_display.sync()` before exit** (async
   protocol — exiting early risks the unlock never landing).
5. Handle `finished` (denied / superseded) → fall back to the Freya locker.
6. Hard rule: **if the client dies while locked, the session can stay locked
   permanently.** Every exit path must unlock first. No `process::exit`
   mid-lock, sync-on-exit, panic hook that attempts unlock.

## Crate shape: `services/session-lock` (bin `finick-lock`)

- Compositor conn: `wayland-client` + `calloop` event loop; bind
  compositor/shm/output/seat/keyboard + session-lock manager.
- Rendering: SHM buffers via memfd, software raster. Reuse in-tree crates:
  `tiny-skia` for drawing, `image` for wallpaper decode, `ab_glyph` for
  glyphs (font bytes discovered at runtime from system font dirs, dots-only
  fallback if none found).
- Input: `wl_keyboard` keymap fd → `xkbcommon` crate for keysyms;
  Enter/Backspace/Esc + text buffer. No pointer needed.
- Auth: reuse `system::verify_password` as-is (sudo/su/shadow chain).
- UI, phased: solid bg + password dots first; wallpaper + clock +
  per-output + scale + hotplug later.
- The Freya locker retires as the primary UI — it can never receive input
  under session lock. Keep it only as the `finished`-event fallback.

## Integration

- `trigger_lock()` / Super+L / panel button → `finick-lock`;
  `locker_binary()` resolution order gains it first.
- Flake: new wrapper bin. HM module: swap the bind target, drop the
  window-rule hacks (unneeded — lock surfaces aren't windows).
- Overlay runtime binder + finickd respawn supervisor become obsolete for
  security (keep supervisor only as crash-respawn backstop during
  transition).

## Testing

Never on a live session. Spare TTY: log in on F3, start a minimal Hyprland,
lock/unlock there. Kill -9 the locker while locked at least once to confirm
the behavior is understood, not surprising.

## Phases

Status 2026-09-29: **P0 done** in `services/session-lock` (bin `finick-lock`):
lock → per-output lock surfaces → configure/ack/commit blank SHM →
`locked` → `--timeout`/SIGINT/SIGTERM → `unlock_and_destroy` + `sync` wait,
`finished` → exit 42 (Freya fallback), panic hook attempts unlock.
`cargo check -p session-lock` clean; headless here so the TTY test below is
still required before P1. Next: P1 input (keyboard + xkbcommon + verify).

Status 2026-09-29: **P1 done** in `services/session-lock` (+ `src/finger.rs`):
seat/keyboard via `wl_seat` capabilities, keymap fd → `xkbcommon-dl`
(dlopen, no link-time dep; gracefully fingerprint-only if missing),
password buffer with Enter/Backspace/Esc, Enter → `system::verify_password`
on a thread → same unlock path; wrong password stays locked (red dots).
Fingerprint via fprintd blocking-D-Bus thread (`Claim`/`VerifyStart`/
`VerifyStatus`, rearm on no-match, `Release` on exit) → match unlocks;
no device/enrolled/denied → password-only. Frame is bg + finger ring +
password dots (shapes only, text waits for P2 fonts). Unit-tested painter.
Still required: TTY test (lock → type → finger → kill -9 behavior).

Status 2026-09-29: **other login methods via PAM** (`system::authenticate_user`,
`devenv.nix` + `pkgs.linux-pam`): the locker now auths through the PAM stack
first (`finick-lock` service when shipped, else `login`), so whatever the
admin installed — password, pam_fprintd, face (howdy), u2f, smartcard,
systemd-homed — works with no per-method code; text prompts are answered
from the typed buffer, touch/presence modules need no conversation. Legacy
sudo/su/shadow chain remains as fallback. Deliberately **no `pam` crate**:
its bindgen step is flaky in this workspace (clang-sys runtime loader vs
nix libclang versions); libpam is dlopened directly with `libc` (same
pattern as xkbcommon-dl), missing lib → fallback. Unit-tested (backdoors,
real-stack negative, service selection). P3 still owes `/etc/pam.d/finick-lock`
packaging.

Status 2026-09-29: **P2 done** (`services/session-lock/src/render.rs` + wiring):
wallpaper from finick settings (file → cover-fit + dim, `preset:N`, `#hex`,
solid fallback; `FINICK_LOCK_WALLPAPER` override), clock + date via runtime
font discovery (system font dirs, scored sans-first, `ab_glyph` raster,
shapes-only when absent) with minute-boundary refresh timer, per-output
`wl_output` scale (buffer × scale + `set_buffer_scale`, redraw on change;
hotplug was already covered). Hints under the dots (finger state /
"Checking…"). Unit-tested (shapes, hex, cover-dim, font walk).
Still required: TTY test with a real wallpaper + HiDPI output.

| Phase | Work | Est |
| ----- | ---- | --- |
| P0 skeleton | crate, registry, lock → `locked` → `unlock_and_destroy` roundtrip, blank surfaces, sync-on-exit, `finished` fallback | 2–3 days |
| P1 input | keyboard + xkbcommon, password buffer, verify, dots rendering | 3–4 days |
| P2 polish | wallpaper, clock + fonts, per-output / scale / hotplug | ~1 week |
| P3 integrate | rewire binds, packaging, retire old locker tricks | 2–3 days |

## Risks

- Client death while locked bricks the session (mitigate: careful exit
  paths, sync, panic-hook unlock attempt).
- `xkbcommon` crate linking against the nix-provided libxkbcommon —
  verify at P1, fallback is keysym tables by hand.
- Font discovery: manual system-font-dir scan vs adding a `fontdb` dep.

## Open questions

1. Confirm Hyprland exposes the manager to unprivileged clients (hyprlock
   working unprivileged says yes — verify on TTY).
2. `xkbcommon` linking (see risks).
3. Font discovery approach (see risks).
