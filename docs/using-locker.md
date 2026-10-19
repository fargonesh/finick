# Lock screen

`SUPER+L` runs `finick-lock`: the ext-session-lock client first, with the Freya `locker` app as the `finished`-event fallback (resolved via `FINICK_LOCKER_BIN`/`PATH`).

- Quiet screen with wallpaper + clock, wakes on sign-in. No Hyprland window rules apply to the lock surface (it isn't a window), so don't add float/pin rules for it.
- PAM service `finick-lock` comes from `nix/finick-lock.pam` via `security.pam.services.finick-lock` on NixOS. Standalone home-manager setups install it manually (see [Install](./install.md)).
- Test the fallback path explicitly if you change session-lock or PAM config — a lock screen you can't unlock is the worst bug in this repo.
