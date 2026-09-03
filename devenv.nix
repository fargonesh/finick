{ pkgs, lib, config, inputs, ... }: 

{

   packages = [
    pkgs.git
    pkgs.lld
    pkgs.mold
    pkgs.devenv
    pkgs.openssl
    pkgs.xdotool
    pkgs.stdenv.cc
    pkgs.binutils
    pkgs.sqlite
    pkgs.pkg-config
    pkgs.gtk4
    pkgs.gtk4.dev
    pkgs.libadwaita
    pkgs.libadwaita.dev
    pkgs.glib
    pkgs.glib.dev
    pkgs.alsa-lib
    pkgs.alsa-lib.dev
    pkgs.atk
    pkgs.atk.dev
    pkgs.gtk3
    pkgs.gtk3.dev
    pkgs.pango
    pkgs.pango.dev
    pkgs.cairo
    pkgs.cairo.dev
    pkgs.gdk-pixbuf
    pkgs.gdk-pixbuf.dev
    pkgs.webkitgtk_4_1
    pkgs.webkitgtk_4_1.dev
    pkgs.libsoup_3
    pkgs.libsoup_3.dev
    pkgs.librsvg
    pkgs.librsvg.dev
    pkgs.dbus
    pkgs.dbus.dev
    pkgs.llvmPackages.libclang
    pkgs.libv4l
    pkgs.libv4l.dev
    pkgs.libxkbcommon
    pkgs.wayland
    pkgs.vulkan-loader
    pkgs.libglvnd

    # System CLI tools & utilities (shelled out by libs/system & apps)
    pkgs.hyprland        # hyprctl (monitors, devices)
    pkgs.wireplumber     # wpctl (audio volume/mute/status)
    pkgs.networkmanager  # nmcli (wifi, networking)
    pkgs.bluez           # bluetoothctl (bluetooth devices)
    pkgs.util-linux      # rfkill
    pkgs.coreutils       # df, uname, date
    pkgs.procps          # free, uptime
    pkgs.systemd         # timedatectl, systemctl
    pkgs.iproute2        # ip
    pkgs.xdg-utils       # xdg-open
    pkgs.hostname        # hostname
    pkgs.cups            # lpstat, lpoptions, cupsenable, cupsdisable, lpr, cancel (printers)
    pkgs.sane-backends   # scanimage (scanners)
    pkgs.iptables        # iptables (firewall & privacy)
    pkgs.firewalld       # firewall-cmd (firewall & privacy)
    pkgs.tailscale       # tailscale (VPN management)
  ];

  env = {
    LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
    BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${pkgs.glibc.dev}/include -isystem ${pkgs.linuxHeaders}/include";
    LD_LIBRARY_PATH = lib.makeLibraryPath [
      pkgs.libxkbcommon
      pkgs.wayland
      pkgs.vulkan-loader
      pkgs.libglvnd
    ];
  };

  enterShell = ''
    export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"
    export BINDGEN_EXTRA_CLANG_ARGS="-isystem ${pkgs.glibc.dev}/include -isystem ${pkgs.linuxHeaders}/include"
    export LD_LIBRARY_PATH="${lib.makeLibraryPath [ pkgs.libxkbcommon pkgs.wayland pkgs.vulkan-loader pkgs.libglvnd ]}:$LD_LIBRARY_PATH"
    echo ""
    echo "Rust toolchain: $(rustc --version)"
    echo ""
  '';

  # Services in ./services (e.g. index, settings-daemon)
  processes = lib.mapAttrs' (name: _:
    lib.nameValuePair name {
      exec = "cargo run -p ${name}";
    }
  ) (lib.filterAttrs (_name: type: type == "directory") (builtins.readDir ./services));

  scripts = {
    devtools.exec = "if [ ! -f ./.devenv/state/cargo-install/bin/freya-devtools-app ]; then cargo install --git https://github.com/marc2332/freya freya-devtools-app --root ./.devenv/state/cargo-install; fi; ./.devenv/state/cargo-install/bin/freya-devtools-app \"$@\"";
    settings.exec = "cargo run -p settings -- \"$@\"";
    settings-daemon.exec = "cargo run -p settings-daemon -- \"$@\"";
    files.exec = "cargo run -p files -- \"$@\"";
    finickctl.exec = "cargo run -p finickctl -- \"$@\"";
  };

  languages.rust = {
    enable = true;
    channel = "nightly";
  };

  pre-commit.hooks = {
    rustfmt.enable = true;
    clippy.enable = true;
    cargo-check.enable = true;
  };
}