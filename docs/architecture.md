# Architecture

Four layers, one direction. Backend → daemon → store → UI; writes return as intents with confirm/deny + rollback.

- **Backends** (`libs/system`, `HyprlandBackend`, `nmcli`, `bluetoothctl`, `wpctl`, `lpstat`): touch the machine.
- **Daemons**: `services/finickd` owns `SettingKey`/`SettingValue` truth (`set_and_apply` accepts or denies with a reason); `services/index` answers file search; `polkit-agent` and `session-lock` cover auth and locking.
- **Transport** (`libs/ipc`, `libs/protocol`): typed requests including `index::Request::Search`, settings get/set, `ModalRequest` (`PamAuth`, `WifiPassword`, `BluetoothPair`).
- **Apps** (`apps/*` on Freya): local `use_state` for filter/selection/arm-confirm only; shared truth lives in `SettingsStore` (`apps/settings/src/state.rs`). Components stay synchronous — one-shot fetch on mount via `use_hook` + `spawn` + `spawn_blocking`, streaming via channel + `set_if_modified`, search with generation counters, polling tied to mount lifetime.
- **Config/theme** (`libs/config`): accent names → hex, wallpaper, border; every app follows the same tokens (see [Design system](./design_system.md)).

Binaries: `settings`, `files`, `launcher`, `overlay`, `store`, `finick-lock`, `locker` (fallback), `finickd`, `index`, `finickctl`.

## Dev screenshots (`--screenshot`)

Every app (`settings`, `files`, `overlay`, `launcher`, `locker`, `store`) accepts a `--screenshot` debug flag for product shots and visual checks. It waits for first paint, grabs the screen, prints the path, and exits. Capture runs through `grim`, falling back to `hyprshot -m output`, then X11 tools (`import -window root`, `scrot`, `maim`) when running under X.

```sh
devenv shell settings --screenshot
devenv shell settings --screenshot --screenshot-to /tmp/settings-wifi.png --screenshot-delay-ms 2000
devenv shell settings wifi --screenshot
```

Defaults: output `/tmp/<app>-screenshot.png`, 1500 ms delay. `devenv.nix` ships `grim`, `slurp`, `wl-screenrec`, `hyprshot`, and `wl-clipboard`, so this works inside `devenv shell`. Outside it you need `grim` or `hyprshot` on `PATH` yourself; without either the app exits 1 with `[settings] screenshot failed: no screenshot tool found (install grim or hyprshot)`.

### Isolated virtual display (`--screenshot-virtual`)

`--screenshot` captures whatever is on screen. For clean, reproducible
product shots (no host desktop, works headless/CI), add `--screenshot-virtual`:

```sh
devenv shell settings --screenshot-virtual --screenshot-to /tmp/settings-wifi.png
devenv shell settings wifi --screenshot-virtual --screenshot-size 1920x1080
```

The harness boots an isolated Xvfb server (default `1280x800x24`, override
with `--screenshot-size <WxH>`), re-executes the app under it via X11,
captures with `import`/`scrot`/`maim`, then tears down Xvfb and exits with
the child's status. Aliases: `--virtual-display`, `--screenshot-headless`.
`devenv.nix` ships `xorgserver` (Xvfb), `xauth`, `scrot`, `maim`, and
`imagemagick` (`import`) for this path.

### Window geometry (`--window-size`)

`--window-size <WxH>` (alias `--window-geometry`) pins the main window of
`settings`, `files`, `store`, and `launcher` to deterministic geometry.
It is orthogonal to `--screenshot`: use it alone for headed layout
debugging, or combine it for reproducible shots.

```sh
devenv shell settings wifi --window-size 1600x900
devenv shell settings --screenshot --screenshot-size=1920x1080 --window-size=1600x900
devenv shell settings --screenshot-virtual --screenshot-to /tmp/shot.png --window-size 1280x800
```

`--screenshot-size` controls the virtual display; `--window-size` controls
the app window inside it. `overlay` panels and `locker` keep fixed /
fullscreen geometry, so the flag does not apply to them.

### Debugging uses

The same flags are the GUI debugging harness, not just the product-shot
pipeline (`web/public/shots`):

- Deep-link straight to the surface under test: `settings wifi`, `store
  installed --search gimp`, `overlay --page screenshot`.
- Pin geometry while iterating on layout: `--window-size 1600x900`.
- Capture headlessly (CI, containers, agents without a display):
  `--screenshot-virtual`, optionally with `--screenshot-delay-ms` for
  slow first paint and `--screenshot-to` for the output path.
