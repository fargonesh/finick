# NixOS / home-manager module

Source: `nix/home-manager.nix`. Exposed as `homeManagerModules.default`, `homeManagerModules.finick`, and `nixosModules.default`.

```nix
programs.finick = {
  enable = true;
  package = inputs.finick.packages.${pkgs.system}.default; # default
  overlay.enable = true;    # finick-overlay user service
  topbar.enable = false;    # deprecated, use overlay
  settings.enable = true;
  files.enable = true;
  daemon.enable = true;     # finickd user service
  store.enable = true;
  store.extraPackages = [ "firefox" "vlc" ];        # nixpkgs attrs
  store.flatpaks = [ "com.spotify.Client" ];        # flatpak IDs
};
```

Services wired when `programs.finick.enable`:

- `systemd.user.services.finickd` → `${package}/bin/finickd`, wanted by `graphical-session.target`
- `systemd.user.services.finick-overlay` → `${package}/bin/overlay` (when `overlay` or `topbar` enabled), restart on failure, wanted by `graphical-session.target` and `hyprland-session.target`
- `systemd.user.services.finick-index` → `${package}/bin/index` (when `files` enabled)

Hyprland extras:

- `bind = SUPER, L, exec, finick-lock` and `bind = SUPER, SPACE, exec, launcher`
- `windowrulev2` float+pin+noanim+noshadow+noborder for `launcher`, `overlay`, `overlay-panel`, `overlay-modal`, `overlay-clipboard`, `overlay-screenshot`, `overlay-notifications` (plus `rounding 20` on launcher, panel, clipboard)

Non-NixOS note: the `security.pam.services.finick-lock` branch only applies in a NixOS module context. Standalone home-manager users install `nix/finick-lock.pam` manually:

```sh
sudo install -m 644 nix/finick-lock.pam /etc/pam.d/finick-lock
```
