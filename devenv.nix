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
    pkgs.xorg.xorgserver # Xvfb (isolated virtual display for --screenshot-virtual)
    pkgs.xorg.xauth # xauth (Xvfb display auth tooling)
    pkgs.scrot # scrot (X11 screenshot fallback for --screenshot-virtual)
    pkgs.maim # maim (X11 screenshot fallback for --screenshot-virtual)

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
    unlock.exec = ''
      set -u
      FORCE=0
      for a in "$@"; do
        case "$a" in
          --force|--yes|-y) FORCE=1 ;;
          -h|--help)
            echo "Usage: unlock [--force]"
            echo ""
            echo "Safe recovery when a lock client died and the session looks stuck."
            echo "Default prints state and exits without changing anything."
            echo "--force sends SIGTERM to finick-lock/locker, clears a stale"
            echo "finick-locker.lock only after the owning PID is verified dead,"
            echo "wakes the display via hyprctl dpms on, and restarts finick-overlay."
            echo "Never uses misc:allow_session_lock_restore (removed upstream)."
            exit 0
            ;;
        esac
      done
      say() { printf '%s\n' "$*"; }
      have() { command -v "$1" >/dev/null 2>&1; }
      # hyprctl exits 0 even when a lua dispatch errors, so check output text.
      _hypr_ok() {
        out="$("$@" 2>&1)" || return 1
        if printf '%s' "$out" | grep -qi "error\|fail\|nil value\|unknown request\|not set\|is hyprland running"; then
          return 1
        fi
        return 0
      }
      hypr_dpms_on() {
        if ! have hyprctl; then return 1; fi
        # Lua-first (Hyprland >=0.55 lua provider), then legacy hyprlang.
        # Try the env instance first, then -i 0 for the TTY case where
        # HYPRLAND_INSTANCE_SIGNATURE is unset.
        if _hypr_ok hyprctl dispatch 'hl.dsp.dpms("on")'; then return 0; fi
        if _hypr_ok hyprctl eval 'hl.dispatch(hl.dsp.dpms("on"))'; then return 0; fi
        if _hypr_ok hyprctl dispatch dpms on; then return 0; fi
        if _hypr_ok hyprctl -i 0 dispatch 'hl.dsp.dpms("on")'; then return 0; fi
        if _hypr_ok hyprctl -i 0 eval 'hl.dispatch(hl.dsp.dpms("on"))'; then return 0; fi
        if _hypr_ok hyprctl -i 0 dispatch dpms on; then return 0; fi
        return 1
      }
      hypr_responsive() {
        if ! have hyprctl; then return 1; fi
        if hyprctl monitors >/dev/null 2>&1; then return 0; fi
        if hyprctl -i 0 monitors >/dev/null 2>&1; then return 0; fi
        return 1
      }
      hypr_locked_state() {
        # Prints true/false, or returns 1 when unknown.
        for prefix in "hyprctl" "hyprctl -i 0"; do
          # shellcheck disable=SC2086
          out="$($prefix locked 2>&1)" || continue
          if printf '%s' "$out" | grep -qi "error\|fail\|nil value\|unknown request\|not set\|is hyprland running"; then
            continue
          fi
          state="$(printf '%s' "$out" | tr '[:upper:]' '[:lower:]' | grep -Eo 'true|false' | head -n1)"
          if [ -n "$state" ]; then
            printf '%s' "$state"
            return 0
          fi
        done
        return 1
      }
      list_pids() {
        pids=""
        if have pgrep; then
          pids="$(pgrep -x finick-lock 2>/dev/null; pgrep -x locker 2>/dev/null)"
        fi
        for p in $pids; do
          [ "$p" = "$$" ] && continue
          [ "$p" = "$PPID" ] && continue
          if [ -r "/proc/$p/cmdline" ]; then
            if tr '\0' ' ' < "/proc/$p/cmdline" | grep -Eq '(^|/)(finick-lock|locker)( |$)'; then
              printf '%s\n' "$p"
            fi
          fi
        done
      }
      lockfile=""
      for cand in "''${XDG_RUNTIME_DIR:-/tmp}/finick-locker.lock" /tmp/finick-locker.lock; do
        if [ -f "$cand" ]; then lockfile="$cand"; break; fi
      done
      say "[unlock] lock PIDs:"
      found=0
      # shellcheck disable=SC2046
      for pid in $(list_pids); do
        found=1
        say "  pid $pid: $(tr '\0' ' ' < /proc/$pid/cmdline 2>/dev/null)"
      done
      [ "$found" -eq 0 ] && say "  none"
      if [ -n "$lockfile" ]; then
        say "[unlock] lockfile: $lockfile (pid file: $(cat "$lockfile" 2>/dev/null))"
      else
        say "[unlock] lockfile: none"
      fi
      if have hyprctl; then
        if hypr_responsive; then
          say "[unlock] hyprland: responsive"
          if locked_state="$(hypr_locked_state)"; then
            say "[unlock] session locked: $locked_state"
          else
            say "[unlock] session locked: unknown"
          fi
        else
          say "[unlock] hyprland: NOT responding to hyprctl (try from the graphical session or with HYPRLAND_INSTANCE_SIGNATURE set)"
        fi
      else
        say "[unlock] hyprland: hyprctl not on PATH"
      fi
      if [ "$FORCE" -ne 1 ]; then
        say "[unlock] dry run: pass --force to TERM lock clients, clear a stale lockfile, dpms on, restart finick-overlay"
        exit 0
      fi
      # shellcheck disable=SC2046
      for pid in $(list_pids); do
        if tr '\0' ' ' < /proc/$pid/cmdline 2>/dev/null | grep -Eq '(^|/)finick-lock( |$)'; then
          say "[unlock] TERM finick-lock pid $pid (orderly unlock)"
          kill -TERM "$pid" 2>/dev/null || true
        else
          say "[unlock] TERM locker pid $pid"
          kill -TERM "$pid" 2>/dev/null || true
        fi
      done
      for i in 1 2 3 4 5 6 7 8 9 10; do
        # shellcheck disable=SC2046
        remaining="$(list_pids)"
        [ -z "$remaining" ] && break
        sleep 0.5
      done
      # shellcheck disable=SC2046
      still="$(list_pids)"
      if [ -n "$still" ]; then
        say "[unlock] still alive after TERM (not escalating to KILL on a protocol lock): $still"
        say "[unlock] switch to a TTY and inspect before doing anything else"
        exit 1
      fi
      if [ -n "$lockfile" ] && [ -f "$lockfile" ]; then
        owner="$(cat "$lockfile" 2>/dev/null | tr -d ' \t\n')"
        stale=1
        if [ -n "$owner" ] && [ -r "/proc/$owner/cmdline" ]; then
          if tr '\0' ' ' < "/proc/$owner/cmdline" | grep -Eq '(^|/)(finick-lock|locker)( |$)'; then
            stale=0
          fi
        fi
        if [ "$stale" -eq 1 ]; then
          rm -f "$lockfile" && say "[unlock] removed stale $lockfile"
        else
          say "[unlock] lockfile owner $owner still alive, keeping it"
        fi
      fi
      if have systemctl; then
        if systemctl --user start finick-overlay 2>/dev/null; then
          say "[unlock] finick-overlay started"
        else
          say "[unlock] finick-overlay start attempted (check systemctl --user status finick-overlay)"
        fi
      else
        say "[unlock] systemctl not found, start finick-overlay manually"
      fi
      if hypr_dpms_on; then
        say "[unlock] hyprctl dpms on: display woken"
      else
        if have hyprctl; then
          say "[unlock] hyprctl dpms on: FAILED (run hyprctl -i 0 dispatch 'hl.dsp.dpms(\"on\")' from the session)"
        else
          say "[unlock] hyprctl dpms on: skipped (hyprctl not on PATH)"
        fi
      fi
      say "[unlock] done: verify with hyprctl monitors and pgrep -x finick-lock"
    '';
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
