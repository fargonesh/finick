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

        buildDeps = with pkgs; [
          rustToolchain
          stdenv.cc
          binutils
          pkg-config
          git
          python3
          mold
          lld
        ];

        libDeps = with pkgs; [
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
          libsoup_3
          librsvg
          dbus
          libxkbcommon
          wayland
          vulkan-loader
          libglvnd
          sqlite
          stdenv.cc.cc.lib
        ];

        cliTools = with pkgs; [
          hyprland
          wireplumber
          networkmanager
          bluez
          util-linux
          coreutils
          procps
          systemd
          iproute2
          xdg-utils
          hostname
        ];

        finick = pkgs.symlinkJoin {
          name = "finick";
          paths = (map (bin:
            pkgs.writeShellScriptBin bin ''
              set -e
              REPO="$HOME/Documents/Projects/finick"
              if [ ! -f "$REPO/Cargo.toml" ]; then
                REPO="/home/flora.hill@rmhedge.com/Documents/Projects/finick"
              fi
              if [ ! -f "$REPO/Cargo.toml" ]; then
                REPO="/home/519374d0-07bf-447b-8c84-f74f817bdc12/Documents/Projects/finick"
              fi
              export PATH="${pkgs.lib.makeBinPath (buildDeps ++ cliTools)}:$PATH"
              export PKG_CONFIG_PATH="${pkgs.lib.makeSearchPathOutput "dev" "lib/pkgconfig" libDeps}''${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
              export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath libDeps}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
              export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"
              export BINDGEN_EXTRA_CLANG_ARGS="-isystem ${pkgs.glibc.dev}/include -isystem ${pkgs.linuxHeaders}/include"
              export CARGO_TARGET_DIR="''${CARGO_TARGET_DIR:-$HOME/.cache/finick/target}"
              export CARGO_HOME="''${CARGO_HOME:-$HOME/.cargo}"
              exec ${rustToolchain}/bin/cargo run -p ${bin} --manifest-path "$REPO/Cargo.toml" -- "$@"
            '')
            [ "topbar" "settings" "files" "settings-daemon" "index" "finickctl" ]) ++ [ desktopEntries ];
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
