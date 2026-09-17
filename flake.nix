{
  description = "Finick — Hyprland desktop environment (overlay, settings, files)";

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
            wl-clipboard
          ];
        };

        desktopEntries = pkgs.stdenv.mkDerivation {
          pname = "finick-desktop-entries";
          version = "0.1.0";
          src = ./.;
          nativeBuildInputs = with pkgs; [ desktop-file-utils ];
          installPhase = ''
            mkdir -p $out/share/applications
            cat > $out/share/applications/finick-settings.desktop <<'EOF'
            [Desktop Entry]
            Name=Finick Settings
            Comment=System settings for Finick desktop
            Exec=settings
            Icon=preferences-system
            Terminal=false
            Type=Application
            Categories=Settings;System;X-GNOME-Settings-Panel;
            Keywords=Settings;Preferences;System;
            StartupWMClass=settings
            EOF
            cat > $out/share/applications/finick-files.desktop <<'EOF'
            [Desktop Entry]
            Name=Finick Files
            Comment=File manager for Finick
            Exec=files
            Icon=system-file-manager
            Terminal=false
            Type=Application
            Categories=System;FileManager;FileManager;
            Keywords=Files;File Manager;Browser;
            StartupWMClass=files
            MimeType=inode/directory;application/x-gnome-saved-search;
            EOF
            desktop-file-validate $out/share/applications/*.desktop
          '';
        };

        finick = pkgs.symlinkJoin {
          name = "finick";
          paths = (map (bin:
            let
              cargoArgs =
                if bin == "overlay" then "-p overlay --bin overlay"
                else if bin == "topbar" then "-p overlay --bin overlay"
                else "-p ${bin}";
            in
            pkgs.writeShellScriptBin bin ''
              set -e
              REPO="$HOME/Documents/Projects/finick"
              if [ ! -f "$REPO/Cargo.toml" ]; then
                REPO="/home/flora.hill@rmhedge.com/Documents/Projects/finick"
              fi
              if [ ! -f "$REPO/Cargo.toml" ]; then
                REPO="/home/519374d0-07bf-447b-8c84-f74f817bdc12/Documents/Projects/finick"
              fi
              export CARGO_TARGET_DIR="''${CARGO_TARGET_DIR:-$HOME/.cache/finick/target}"
              export CARGO_HOME="''${CARGO_HOME:-$HOME/.cargo}"
              ORIG_DIR="$PWD"
              cd "$REPO"
              DEVENV_BIN="${pkgs.devenv}/bin/devenv"
              if command -v devenv >/dev/null 2>&1; then
                DEVENV_BIN="devenv"
              fi
              exec "$DEVENV_BIN" --quiet shell -- bash -c 'cd "$1"; shift; exec cargo run ${cargoArgs} --manifest-path "'"$REPO"'/Cargo.toml" -- "$@"' bash "$ORIG_DIR" "$@"
            '')
            [ "overlay" "topbar" "settings" "files" "finickd" "index" "finickctl" ]) ++ [ desktopEntries ];
          meta.mainProgram = "overlay";
        };
      in
      {
        packages = {
          default = finick;
          finick = finick;
        };

        apps = {
          default = flake-utils.lib.mkApp { drv = finick; exePath = "/bin/overlay"; };
          overlay = flake-utils.lib.mkApp { drv = finick; exePath = "/bin/overlay"; };
          topbar = flake-utils.lib.mkApp { drv = finick; exePath = "/bin/overlay"; };
          settings = flake-utils.lib.mkApp { drv = finick; exePath = "/bin/settings"; };
          files = flake-utils.lib.mkApp { drv = finick; exePath = "/bin/files"; };
          finickd = flake-utils.lib.mkApp { drv = finick; exePath = "/bin/finickd"; };
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
