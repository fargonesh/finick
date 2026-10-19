# Troubleshooting

## Services not running

```sh
systemctl --user status finickd finick-overlay finick-index
journalctl --user -u finickd -e
```

Rebuild + relogin after module changes; binds and user services only pick up on a fresh graphical session.

## Toggle flips back / "Managed by NixOS"

That's rollback working: the daemon denied the write. Read the inline `last_error`, check policy/lock state, and don't force local state — fix the backend or policy instead.

## Launcher won't toggle

`finickctl -p launcher -s toggle` uses `/tmp/finick-launcher.lock`. Stale lock after a crash: remove the file and start fresh. Ensure `launcher` is on `PATH` or the `hyprctl dispatch exec` fallback can reach it.

## Notifications do nothing

`finickctl -p notify` needs the notification daemon on the user bus. "Failed to run busctl" means the daemon isn't there, not that your message was wrong.

## Store apply fails

Open Details, Copy log, fix the named attr, Retry. `search` first to confirm the nixpkgs attr or flatpak ID exists. `link` re-ensures the generated `apps.nix` import before `apply`.

## Lock screen

If `finick-lock` drops to the Freya fallback unexpectedly, check `FINICK_LOCKER_BIN`/`PATH` and `/etc/pam.d/finick-lock`. Never ship a lock change you haven't unlocked from.

## Screenshots

`[settings] screenshot failed: no screenshot tool found (install grim or hyprshot)` means neither binary was on `PATH`. Inside the repo run it through `devenv shell` (`devenv shell settings --screenshot`; see [Architecture](./architecture.md) for the full flag), which provides `grim`, `slurp`, `wl-screenrec`, `hyprshot`, and `wl-clipboard`. Outside devenv, `which grim` should print something; on NixOS add the tools to your system or home packages.

The overlay screenshot utility fails the same way, with more specific errors: `grim is required for screenshots but was not found` (stills), `slurp is required for region selection but was not found` (Region button only; Fullscreen keeps working), `No screencast recorder found (install wl-screenrec or wf-recorder)`. Region cancelling with Escape is not an error; it reports `Region selection cancelled` and saves nothing. If the clipboard copy silently skips, `wl-copy` is missing; the PNG in `~/Pictures/Screenshots` is still fine.

## Verify the build

```sh
cargo check -p settings -p files -p overlay -p launcher -p store
npm --prefix web run build
```

Treat new warnings as unfinished behavior, not noise.
