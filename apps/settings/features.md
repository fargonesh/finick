# Features Implementation Status

This document tracks the implementation status of all settings pages and features in Finick Settings, distinguishing between **Working (Real System Integration)**, **Partially Implemented / Mock Data**, and **Not Implemented**.

---

## Legend
- **`[x] Working`**: Real system integration via CLI/IPC/sysfs; reads live system state and/or executes real system changes.
- **`[~] Partial / Mock`**: Functional UI with in-memory state or mock data fallback when system tools are unavailable; does not fully persist or enforce system-wide configuration.
- **`[ ] Not Implemented`**: Feature is not yet built or wired up.

---

## Architecture & Integration
- [~] **Background Daemon**: `services/finickd` exists as a skeleton/stub with a mock tick loop; not yet actively connected to the UI or system database.
- [x] **NixOS Module**: Declarative module available at `apps/settings/nix/settings-module.nix` exposing `programs.finick.settings` options.
- [ ] **Settings Sync (`nixos-rebuild switch`)**: Declarative NixOS rebuild and automatic configuration generation not yet implemented.
- [x] **Hyprland & NetworkManager Integration**: Native integration with `hyprctl` (monitors, devices, window gaps, borders) and `nmcli` (Wi-Fi radio, scan, connect, delete).

---

## Account & Connectivity
- [x] **Wi-Fi**:
  - **Working**: Live Wi-Fi radio status and toggling via `nmcli radio wifi on/off`; live access point scanning via `nmcli dev wifi`; connection status via `nmcli connection show`; connect (`nmcli dev wifi connect`), disconnect (`nmcli connection down`), and forget network (`nmcli connection delete`).
  - **Partial / Mock**: Falls back to realistic mock networks (`Dark Depths`, `OPTUS_FF7C2CL`, etc.) only if `nmcli` is unavailable or produces no results. Advanced DNS, proxy, and MAC randomization are not yet implemented.
- [x] **Bluetooth**:
  - **Working**: 100% real system integration via `bluetoothctl` and `rfkill`. Live adapter power status (`bluetoothctl show`), paired devices scan (`bluetoothctl devices`), connected device tracking (`bluetoothctl devices Connected`), device connect/disconnect (`bluetoothctl connect/disconnect`), device unpair (`bluetoothctl remove`), and power toggling (`bluetoothctl power on/off`). No mock data used.
- [x] **Network & Interfaces**:
  - **Working**: Queries primary routing interface and active IPv4 gateway via `ip route get 1.1.1.1`; queries interface states from `ip -br link show`.
  - **Not Implemented**: Low-data mode and custom firewall profile switching from the network page.
- [x] **VPN**:
  - **Working**: Detects active WireGuard (`wg*`), Tailscale (`tailscale*`), and TUN/TAP interfaces and their assigned IP addresses via `ip` and `ip route`. Toggling triggers `tailscale up/down` or `nmcli connection up/down`.
  - **Not Implemented**: New manual VPN profile creation wizard (OpenVPN/IPsec configs).

---

## Notifications, Focus & Sound
- [ ] **Notifications**: Application alert styles, banners, badges, lock screen previews, and scheduled summaries.
- [~] **Sound**:
  - **Working**: Real PipeWire/WirePlumber integration via `wpctl`. Master output volume slider runs `wpctl set-volume @DEFAULT_AUDIO_SINK@ <N>%`; mute switch runs `wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle`; default audio sink name parsed live from `wpctl status`.
  - **Not Implemented**: Input levels / microphone gain slider, alert sounds, sound effect themes, and spatial audio configuration.
- [ ] **Focus**: Do Not Disturb, Focus modes (Work, Personal, Sleep), focus filters, and schedule automations.
- [ ] **Screen Time**: App usage limits, downtime, communication limits, and cross-device screen time analytics.

---

## General & System Management
- [x] **About**:
  - **Working**: 100% real system specifications: Hostname (`/etc/hostname` / `hostname`), OS distribution & release (`/etc/os-release`), Linux kernel version (`uname -r`), System uptime (`uptime -p` / `/proc/uptime`), CPU architecture (`uname -m`), and Total installed RAM (`/proc/meminfo`).
  - **Not Implemented**: Hardware warranty / support coverage (non-applicable on custom Linux).
- [ ] **Software Update**: OS upgrades, channel management, and automatic update cadences.
- [x] **Storage**:
  - **Working**: Real storage queries via `df -hP`. Parses root filesystem size, used space, available space, percentage, and all physical mounted partitions while filtering virtual filesystems (`tmpfs`, `devtmpfs`, `overlay`).
  - **Not Implemented**: Storage recommendations and automatic cache cleanup tools.
- [ ] **Login Items & Extensions**: Startup applications, systemd user services, and desktop background daemons.
- [~] **Language & Region**:
  - **Working**: Reads system locale (`locale`), `localectl status`, current keyboard layout (`X11 Layout`), regional measurement conventions, and installed system locales (`locale -a` / `localectl list-locales`). Dynamically renders real localized dates via `LC_ALL=<locale> date`.
  - **Partial / Mock**: Preferred languages priority ordering and spellcheck/autocorrect switches operate in-memory; does not persist changes to `/etc/locale.conf` via `localectl set-locale`.
- [x] **Date & Time**:
  - **Working**: Queries current date, 12-hour/24-hour time via `date`; queries timezone, UTC time, RTC time, and NTP sync status via `timedatectl`. Automatic network time synchronization switch executes real `timedatectl set-ntp true/false`.
  - **Partial**: 24-hour time format switch is an application-level display preference. Custom manual timezone picker and custom NTP server configuration are not yet implemented.
- [ ] **Sharing**: Local file sharing (Samba/NFS), screen sharing (Wayland VNC/RDP), SSH remote login, and printer sharing.
- [ ] **Transfer or Reset**: Migration tools, profile import/export, factory reset.

---

## Appearance, Display & Desktop
- [~] **Appearance**:
  - **Working**: Real Hyprland window manager integration via `hyprctl`. Queries live `general:gaps_in`, `general:gaps_out`, and `general:border_size`; sliders execute live changes via `hyprctl keyword general:gaps_in <N>`, `hyprctl keyword general:gaps_out <N>`, and `hyprctl keyword general:border_size <N>`.
  - **Partial / Mock**: Dark/Light mode toggle and Accent Color selection dynamically re-theme the Finick Settings app in-memory; does not currently rewrite GTK/Qt themes or Hyprland border color configurations to disk.
- [ ] **Accessibility**: High-contrast mode, screen reader support, large text scaling, cursor size.
- [ ] **Control Center**: Panel/bar icon visibility toggles.
- [ ] **Desktop & Dock**: Window manager layout configurations, dock sizing, workspaces layout, and widget placement.
- [~] **Displays**:
  - **Working**: Queries real connected monitors, display model names, resolutions, refresh rates, and active scale factors using `hyprctl monitors -j`.
  - **Not Implemented**: Changing display resolution, refresh rate, orientation, or virtual layout arrangement from the UI.
- [ ] **Wallpaper**: Wallpaper selection, directory browser, and transition timers.
- [ ] **Screen Saver / Lock**: Idle timeout delays, visualizers, lock screen messages.

---

## Privacy, Security & Power
- [~] **Battery / Power**:
  - **Working**: Battery telemetry read directly from Linux sysfs (`/sys/class/power_supply/BAT*`), parsing real capacity percentage and charging/discharging state.
  - **Partial / Mock**: Power profile indicator ("Balanced") is static; `power-profiles-daemon` / `tlp` profile switching and sleep timers are not yet wired up.
- [ ] **Lock Screen**: Display sleep delay, password grace period, and lock screen widgets.
- [ ] **Login Password**: User password change dialog (`passwd`).
- [~] **Users & Groups (Accounts)**:
  - **Working**: Real human user detection parsed directly from `/etc/passwd`, filtering accounts with UIDs between 1000 and 59999, extracting usernames, user IDs, and default login shells.
  - **Not Implemented**: User creation (`useradd`), password management, avatar customization, and administrator privilege grants (`wheel` / `sudo`).
- [~] **Privacy & Security**:
  - **Working**: Real firewall status detection via `ufw status`, `firewall-cmd --state`, or `iptables -L -n`; real video device discovery via `/dev/video*`; real microphone source detection via `wpctl status`.
  - **Partial / Mock**: Toggles for Camera, Microphone, Location, and Application Sandboxing are in-memory UI toggles; they do not currently alter PipeWire permissions, kernel device permissions, or Flatpak portal permissions.
  - **Not Implemented**: Full disk encryption management (LUKS/cryptsetup).

---

## Inputs & Peripherals
- [~] **Mouse & Keyboard**:
  - **Working**: Live pointer device and keyboard detection via `hyprctl devices -j`, identifying connected device names and active keymaps.
  - **Not Implemented**: Pointer acceleration curves, scroll sensitivity, natural scrolling toggle, and key repeat delay/rate configuration via `hyprctl keyword input:*`.
- [ ] **Trackpad**: Gestures, tap-to-click, and edge scrolling.
- [ ] **Game Controllers**: Paired gamepads, button mapping, and vibration calibration.
- [~] **Printers & Scanners**:
  - **Working**: Reads real installed CUPS printers (`lpstat -p`), default destination (`lpstat -d`), active queue jobs (`lpstat -o`), and SANE scanners (`scanimage -L`); queries CUPS daemon status via `systemctl is-active cups`. Real actions execute: Pause/Resume printer (`cupsdisable`/`cupsenable`), cancel print job (`cancel <job_id>`), print test page (`lp -d <printer> /etc/os-release`), and start/stop CUPS service (`systemctl start/stop cups`).
  - **Partial / Mock**: If no physical printers or scanners are detected on the system, falls back cleanly to realistic mock hardware entries (`HP LaserJet Pro`, `Brother HL-L2350DW`, `Canon PIXMA TS9120`).

---

## Summary Matrix

| Category / Page | Real System Integration (Works) | Mock Data / In-Memory State | Unimplemented |
| :--- | :--- | :--- | :--- |
| **Wi-Fi** | Radio status, scan networks, connect, disconnect, forget | Mock fallback if `nmcli` missing | Advanced DNS, proxy, MAC randomization |
| **Bluetooth** | Adapter power, scan devices, connect, disconnect, unpair | *None (100% Real)* | Audio codec selection |
| **Network & VPN** | Default gateway & IP (`ip route`), WireGuard/Tailscale/TUN detection & toggling | *None* | Low-data mode, manual VPN config wizard |
| **Sound** | Volume slider, mute switch, default sink (`wpctl`) | *None* | Mic input slider, alert sounds, spatial audio |
| **About** | Hostname, OS, kernel, uptime, CPU arch, RAM | *None (100% Real)* | Warranty/coverage |
| **Storage** | Total, used, free space, all disk partitions (`df -hP`) | Fallback if `df` fails | Storage cleanup tools & recommendations |
| **Language & Region** | Locale, localectl status, keyboard layout, installed locales | Preferred languages order, format toggles | `localectl set-locale` persistence |
| **Date & Time** | Date, 12h/24h time, timezone, RTC, NTP sync status & NTP toggle (`timedatectl`) | 24h display format preference | Timezone picker, custom NTP server |
| **Appearance** | Hyprland gaps & border size (`hyprctl getoption` / `keyword`) | App dark mode & accent color (in-memory) | GTK/Qt theme writing, auto-theme schedule |
| **Displays** | Connected monitors, resolutions, refresh rates, scale (`hyprctl monitors`) | *None* | Resolution / refresh rate configuration UI |
| **Power** | Battery percentage & charging state (`/sys/class/power_supply`) | Power profile selection | Sleep timers, power profiles daemon |
| **Accounts** | Local human users list, UIDs, shells (`/etc/passwd`) | *None* | Add user, change password, sudo privileges |
| **Privacy & Security** | Firewall detection (`ufw`/`iptables`), camera & mic presence | Camera/mic/location permission toggles | LUKS disk encryption, portal enforcement |
| **Mouse & Keyboard** | Connected mice & keyboards detection (`hyprctl devices`) | *None* | Key repeat rate, mouse sensitivity sliders |
| **Printers & Scanners**| CUPS printers, jobs, scanners, pause/resume, cancel job, print test page | Mock fallback if no hardware detected | PPD driver file installer |