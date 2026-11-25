# Settings

`apps/settings`, 23 routes. Sidebar writes `current_route`; top bar Back and `Escape` return to Overview (`Escape` clears search first).

## Pages

Overview, Wi-Fi, Bluetooth, Appearance, Display, Sound, Focus, Notifications, Screen Time, General, Storage, Battery, Mouse, Accessibility, About, Desktop, Date & Time, Privacy, Language, Printers, Accounts, Shortcuts, Default Apps.

## Search and keyboard

- Filter matches title plus per-route keywords.
- Keyboard order follows the visible list (filtered minus About, About last). `ArrowUp`/`ArrowDown` and `Tab` wrap; the focused item shows an accent ring.

## CLI slugs

Routes accept page slugs on the command line / URI tail (`Route::from_slug`, case-insensitive, `_` treated as `-`):

- `overview`, `home`, `""`
- `wifi` (`wi-fi`, `network`, `wireless`), `bluetooth` (`bt`)
- `appearance` (`theme`, `wallpaper`), `display` (`screen`, `monitor`), `sound` (`audio`, `volume`)
- `focus` (`dnd`, `do-not-disturb`), `notifications`, `screen-time`, `general`
- `storage` (`disk`), `battery` (`power`), `mouse` (`touchpad`, `trackpad`)
- `accessibility` (`a11y`), `about` (`info`, `specs`), `desktop` (`dock`, `workspaces`)
- `datetime` (`date-time`, `time`, `clock`, `timezone`)
- `privacy` (`security`, `firewall`), `language` (`locale`, `region`)
- `printers` (`scanners`, `printing`), `accounts` (`users`), `shortcuts` (`keyboard`, `keybindings`, `hotkeys`)
- `default-apps` (`defaults`, `mime`)

Unknown slugs fall back to Overview.

## Backend honesty

Settings reads daemon truth through `SettingsStore` and writes via `set_and_apply`. Denials and IPC errors roll the local value back and set `last_error` inline — the toggle never sticks when the backend refused. Locked keys show "Managed by NixOS" / "Locked by policy". Sliders (volume, brightness, gaps) debounce; nothing fires per pixel.
