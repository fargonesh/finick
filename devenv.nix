{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:
{
  packages = [
    pkgs.cloc
    pkgs.nixfmt

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
    pkgs.linux-pam # libpam headers/lib for the `pam` crate (system::pam_authenticate)
    pkgs.vulkan-loader
    pkgs.libglvnd

    # System CLI tools & utilities (shelled out by libs/system & apps)
    pkgs.nodejs_22
    pkgs.hyprland # hyprctl (monitors, devices)
    pkgs.wireplumber # wpctl (audio volume/mute/status)
    pkgs.networkmanager # nmcli (wifi, networking)
    pkgs.bluez # bluetoothctl (bluetooth devices)
    pkgs.util-linux # rfkill
    pkgs.coreutils # df, uname, date
    pkgs.procps # free, uptime
    pkgs.systemd # timedatectl, systemctl
    pkgs.iproute2 # ip
    pkgs.xdg-utils # xdg-open
    pkgs.hostname # hostname
    pkgs.cups # lpstat, lpoptions, cupsenable, cupsdisable, lpr, cancel (printers)
    pkgs.sane-backends # scanimage (scanners)
    pkgs.iptables # iptables (firewall & privacy)
    pkgs.firewalld # firewall-cmd (firewall & privacy)
    pkgs.tailscale # tailscale (VPN management)
    pkgs.swaybg # swaybg (multi-monitor desktop wallpaper & solid color background)
    pkgs.hyprpaper # hyprpaper (hyprland wallpaper backend)
    pkgs.wl-clipboard # wl-copy, wl-paste (wayland clipboard cli & watcher)
    pkgs.grim # grim (screenshots: --screenshot debug flag + overlay capture)
    pkgs.slurp # slurp (region selection for screenshots/screencasts)
    pkgs.wl-screenrec # wl-screenrec (screencast recording)
    pkgs.hyprshot # hyprshot (screenshot fallback for --screenshot)
    pkgs.grimblast # grimblast (screenshot helper referenced by shortcuts)

    # Media / power / brightness (shelled out by locker, session-lock, overlay, settings)
    pkgs.playerctl # playerctl (locker + session-lock now-playing)
    pkgs.brightnessctl # brightnessctl (display brightness)
    pkgs.upower # upower (battery history)
    pkgs.power-profiles-daemon # powerprofilesctl (power profiles)
    pkgs.pulseaudio # pactl (per-app volumes; wpctl from wireplumber covers the rest)

    # Apps & files (shelled out by store, files, settings)
    pkgs.flatpak # flatpak (store: list/install/update)
    pkgs.poppler-utils # pdftotext, pdftoppm (files: text extraction + thumbnails)
    pkgs.imagemagick # magick/convert (settings: avatar resize)
    pkgs.libcanberra-gtk3 # canberra-gtk-play (settings: alert sound preview)
    pkgs.perlPackages.FileMimeInfo # mimeopen (files: open-with fallback)
    pkgs.gnutar # tar (files: archive handling)
    pkgs.unzip # unzip (files: archive handling)
    pkgs.file # file (type detection)
    pkgs.dconf # dconf (settings backend)
    pkgs.which # which (backend probing: swaybg, hyprsunset, wlsunset, gamestep)
    pkgs.python3 # python3 (auth crypt fallback)
    pkgs.pywal # wal (wallpaper accent sampling, optional)

    # Night light backends (auto-detected in order: hyprsunset, wlsunset, gammastep)
    pkgs.hyprsunset
    pkgs.wlsunset
    pkgs.gammastep
  ];

  env = {
    LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
    BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${pkgs.glibc.dev}/include -isystem ${pkgs.linuxHeaders}/include";
    LD_LIBRARY_PATH = lib.makeLibraryPath [
      pkgs.libxkbcommon
      pkgs.wayland
      pkgs.linux-pam
      pkgs.vulkan-loader
      pkgs.libglvnd
    ];
  };

  enterShell = ''
    export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"
    export BINDGEN_EXTRA_CLANG_ARGS="-isystem ${pkgs.glibc.dev}/include -isystem ${pkgs.linuxHeaders}/include"
    export LD_LIBRARY_PATH="${
      lib.makeLibraryPath [
        pkgs.libxkbcommon
        pkgs.wayland
        pkgs.linux-pam
        pkgs.vulkan-loader
        pkgs.libglvnd
      ]
    }:$LD_LIBRARY_PATH"
    echo ""
    echo "Rust toolchain: $(rustc --version)"
    echo ""
  '';

  # Services in ./services (e.g. index, finickd)
  processes = lib.mapAttrs' (
    name: _:
    lib.nameValuePair name {
      exec = "cargo run -p ${name}";
    }
  ) (lib.filterAttrs (_name: type: type == "directory") (builtins.readDir ./services));

  scripts = {
    cloc.exec = "git ls-files -z | xargs -0 -r ${pkgs.cloc}/bin/cloc -- \"$@\"";
    devtools.exec = "if [ ! -f ./.devenv/state/cargo-install/bin/freya-devtools-app ]; then cargo install --git https://github.com/marc2332/freya freya-devtools-app --root ./.devenv/state/cargo-install; fi; ./.devenv/state/cargo-install/bin/freya-devtools-app \"$@\"";
    settings.exec = "cargo run -p settings -- \"$@\"";
    files.exec = "cargo run -p files -- \"$@\"";
    launcher.exec = "cargo run -p overlay --bin launcher -- \"$@\"";
    locker.exec = "cargo run -p locker -- \"$@\"";
    finick-lock.exec = ''if [ -d services/session-lock ]; then code=0; cargo run -p session-lock -- "$@" || code=$?; if [ "$code" -eq 42 ]; then cargo run -p locker -- "$@"; else exit "$code"; fi; else cargo run -p locker -- "$@"; fi'';
    overlay.exec = "cargo run -p overlay -- \"$@\"";
    store.exec = "cargo run -p store -- \"$@\"";
    finickd.exec = "cargo run -p finickd -- \"$@\"";
    index.exec = "cargo run -p index -- \"$@\"";
    polkit-agent.exec = "cargo run -p polkit-agent -- \"$@\"";
    finickctl.exec = "cargo run -p finickctl -- \"$@\"";
    web.exec = ''
      set -e
      cd web
      if [ ! -d node_modules ]; then
        echo "Installing web deps (vite)..."
        npm install
      fi
      exec npm run dev -- "$@"
    '';
    web-build.exec = ''
      set -e
      cd web
      if [ ! -d node_modules ]; then npm install; fi
      npm run build
      echo "Built to web/dist"
    '';
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
