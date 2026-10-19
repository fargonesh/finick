# Architecture

Four layers, one direction. Backend → daemon → store → UI; writes return as intents with confirm/deny + rollback.

- **Backends** (`libs/system`, `HyprlandBackend`, `nmcli`, `bluetoothctl`, `wpctl`, `lpstat`): touch the machine.
- **Daemons**: `services/finickd` owns `SettingKey`/`SettingValue` truth (`set_and_apply` accepts or denies with a reason); `services/index` answers file search; `polkit-agent` and `session-lock` cover auth and locking.
- **Transport** (`libs/ipc`, `libs/protocol`): typed requests including `index::Request::Search`, settings get/set, `ModalRequest` (`PamAuth`, `WifiPassword`, `BluetoothPair`).
- **Apps** (`apps/*` on Freya): local `use_state` for filter/selection/arm-confirm only; shared truth lives in `SettingsStore` (`apps/settings/src/state.rs`). Components stay synchronous — one-shot fetch on mount via `use_hook` + `spawn` + `spawn_blocking`, streaming via channel + `set_if_modified`, search with generation counters, polling tied to mount lifetime.
- **Config/theme** (`libs/config`): accent names → hex, wallpaper, border; every app follows the same tokens (see [Design system](./design_system.md)).

Binaries: `settings`, `files`, `launcher`, `overlay`, `store`, `finick-lock`, `locker` (fallback), `finickd`, `index`, `finickctl`.

## Dev screenshots (`--screenshot`)

Every app (`settings`, `files`, `overlay`, `launcher`, `locker`, `store`) accepts a `--screenshot` debug flag for product shots and visual checks. It waits for first paint, grabs the screen, prints the path, and exits. Capture runs through `grim`, falling back to `hyprshot -m output`.

```sh
devenv shell settings --screenshot
devenv shell settings --screenshot --screenshot-to /tmp/settings-wifi.png --screenshot-delay-ms 2000
devenv shell settings wifi --screenshot
```

Defaults: output `/tmp/<app>-screenshot.png`, 1500 ms delay. `devenv.nix` ships `grim`, `slurp`, `wl-screenrec`, `hyprshot`, and `wl-clipboard`, so this works inside `devenv shell`. Outside it you need `grim` or `hyprshot` on `PATH` yourself; without either the app exits 1 with `[settings] screenshot failed: no screenshot tool found (install grim or hyprshot)`.
