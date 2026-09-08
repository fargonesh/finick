{
  description = "Finick — Hyprland desktop environment (topbar, settings, files)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane }:
    (flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.rust-bin.nightly.latest.default;

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;
          nativeBuildInputs = with pkgs; [ pkg-config python3 ];
          buildInputs = with pkgs; [
            openssl
            gtk4
            libadwaita
            glib
            alsa-lib
            atk
            gtk3
            pango
            cairo
            gdk-pixbuf
            webkitgtk_4_1
            librsvg
            dbus
            libxkbcommon
            wayland
            vulkan-loader
            libglvnd
            sqlite
          ];
        };

        finick = pkgs.symlinkJoin {
          name = "finick";
          paths = map (bin:
            pkgs.writeShellScriptBin bin ''
              set -e
              REPO="$HOME/Documents/Projects/finick"
              if [ ! -f "$REPO/Cargo.toml" ]; then
                REPO="/home/519374d0-07bf-447b-8c84-f74f817bdc12/Documents/Projects/finick"
              fi
              export CARGO_TARGET_DIR="''${CARGO_TARGET_DIR:-$HOME/.cache/finick/target}"
              export CARGO_HOME="''${CARGO_HOME:-$HOME/.cargo}"
              exec ${pkgs.cargo}/bin/cargo run -p ${bin} --manifest-path "$REPO/Cargo.toml" -- "$@"
            '')
            [ "topbar" "settings" "files" "settings-daemon" "index" "finickctl" ];
          meta.mainProgram = "settings";
        };
      in
      {
        packages = {
          default = finick;
          finick = finick;
        };

        apps = {
          topbar = flake-utils.lib.mkApp { drv = finick; exePath = "/bin/topbar"; };
          settings = flake-utils.lib.mkApp { drv = finick; exePath = "/bin/settings"; };
          files = flake-utils.lib.mkApp { drv = finick; exePath = "/bin/files"; };
          settings-daemon = flake-utils.lib.mkApp { drv = finick; exePath = "/bin/settings-daemon"; };
          index = flake-utils.lib.mkApp { drv = finick; exePath = "/bin/index"; };
          finickctl = flake-utils.lib.mkApp { drv = finick; exePath = "/bin/finickctl"; };
        };

        checks = { inherit finick; };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ finick ];
          packages = with pkgs; [ rustToolchain pkg-config openssl ];
        };
      }
    )) // {
      homeManagerModules.default = ./nix/home-manager.nix;
      homeManagerModules.finick = ./nix/home-manager.nix;
      nixosModules.default = ./nix/home-manager.nix;
    };
}
