# Install

Finick ships as a flake with a home-manager module (also re-exported as `nixosModules.default`).

```nix
{
  inputs.finick.url = "github:fargonesh/finick";
  outputs = { finick, nixpkgs, ... }: {
    nixosConfigurations.yourHost = nixpkgs.lib.nixosSystem {
      modules = [
        finick.nixosModules.default
        { programs.finick.enable = true; }
      ];
    };
  };
}
# then
# nixos-rebuild switch --flake .#yourHost
```

What you get on rebuild:

- Binaries on `PATH`: `settings`, `files`, `launcher`, `overlay`, `store` (app shop), `finick-lock`, `finickd`, `index`, `finickctl`
- User services: `finickd` (settings daemon), `finick-overlay`, `finick-index` (see [NixOS module](./nixos-module.md))
- Hyprland binds: `SUPER+L` → `finick-lock`, `SUPER+SPACE` → `launcher`
- Window rules for `launcher` / `overlay*` classes (float, pin, noanim, noshadow, noborder)
- `inode/directory` defaults to `finick-files.desktop`
- PAM service `finick-lock` from `nix/finick-lock.pam` (NixOS context; on non-NixOS copy it manually to `/etc/pam.d/finick-lock`)

Log out and back in after the first switch so the user services and binds pick up.
