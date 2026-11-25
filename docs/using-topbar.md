# Top bar and overlay

`apps/overlay` (`overlay` binary): panel, quick settings, clipboard, screenshots, notifications, OSD, session dialog, launcher modal, toasts.

## Panel

Hosts sound, Wi-Fi, battery, clipboard, screenshots, and notifications. Live state (wifi / wired / power) polls about every 8s with `set_if_modified` so identical values don't re-render; the clock ticks at 1s. Panels scroll internally instead of growing the window.

## Quick settings and session

Volume/brightness sliders debounce and apply on settle. Power actions (shutdown, restart, logout) arm in place first, then confirm via a `dialog_card` with a `danger_button` — nothing destructive fires on a single incidental click, and the arm reverts after ~5s.

## Clipboard, screenshots, notifications

Clipboard and notification history live in the overlay, with empty states ("No clipboard items") instead of fake controls. Screenshot and OSD surfaces use the shared `overlay-screenshot` / panel window classes from the Nix module. Notification send path is also reachable headless via `finickctl -p notify -s "Summary|Body"` over DBus.

## Screenshot utility
The `overlay-screenshot` surface offers Fullscreen, Region, and Monitor captures through `libs/system/src/capture.rs`. Fullscreen and Monitor need `grim` on `PATH`; Region also needs `slurp` (it draws the picker in your accent color); copy-to-clipboard needs `wl-copy`. Screencast recording, where wired, prefers `wl-screenrec` and falls back to `wf-recorder`.

Stills land in `~/Pictures/Screenshots` (or `$XDG_PICTURES_DIR/Screenshots`) as `Screenshot_YYYY-MM-DD_HH-MM-SS.png`; recordings land in `~/Videos/Screencasts`. A notification fires over DBus after each save. Copy-to-clipboard is on by default and just skips silently when `wl-copy` is missing; the file still saves.

`devenv.nix` ships `grim`, `slurp`, `wl-screenrec`, `hyprshot`, and `wl-clipboard`, so this works inside `devenv shell`. On an installed system those tools have to be on `PATH` separately; the Nix module does not pull them in yet. Missing binary means the button reports it (`grim is required for screenshots but was not found`, `slurp is required for region selection...`) instead of hanging.

## Custom pills

The topbar center shows extendable pills from `~/.config/finick/topbar-pills/*.json`
(override with `XDG_CONFIG_HOME`). Each file runs a command via `sh -c` on its
own interval, so a pill can be any language — bash, python, or JS via node.
Empty output (or a failing command) hides the pill; nothing destructive runs.

```json
{
  "id": "now-playing",
  "label": "♪",
  "command": "playerctl metadata --format '{{artist}} - {{title}}'",
  "on_click": "playerctl play-pause",
  "interval_secs": 2,
  "max_chars": 32
}
```

Fields: `id` (required, sorts the pills), `command` (required), `label`
(optional prefix), `on_click` (optional press command, detached), `interval_secs`
(default 5, clamped 1–3600), `max_chars` (default 32, clamped 8–80), `enabled`
(default true). At most 5 non-empty pills render, each capped at 320px wide.

JS example — `lyrics.js` plus a wrapper JSON with
`"command": "node ~/.config/finick/topbar-pills/lyrics.js"`:

```js
// lyrics.js: print one line for the pill, nothing to hide it.
const { execSync } = require("node:child_process");
try {
  const out = execSync("playerctl metadata --format '{{artist}} - {{title}}'", { timeout: 1500 });
  console.log(out.toString().trim().slice(0, 120));
} catch { /* stay silent: pill hides */ }
```

Toggle the whole row in Settings → Desktop → Topbar icons → Custom pills
(`topbar.show_pills`). Only install pills you trust: commands run as your user.
