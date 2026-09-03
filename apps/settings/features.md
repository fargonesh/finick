# Features Implementation Status

## Architecture & Integration
- [x] Background Daemon: Dedicated `finick-settings-daemon` running in background for Screen Time tracking and data indexing.
- [x] NixOS Module: Expose a `settings-module.nix` that users can inherit for declarative system configuration.
- [ ] Settings Sync: Saving settings triggers `nixos-rebuild switch` or edits local nix configuration files to apply system-level changes.
- [x] Hyprland & NetworkManager: Opinionated tight integration with `hyprctl` and `nmcli`/NetworkManager natively.
## Account & Connectivity
- [x] Wi-Fi: Network selection, known networks, auto-join settings, private Wi-Fi address, and advanced DNS/proxy/hardware configurations.
- [x] Bluetooth: Device pairing, discovery, and connected peripherals.
- [x] Network: Network interfaces (Wi-Fi, Ethernet, Thunderbolt Bridge), VPN connections, firewall filters, locations, and low-data modes.
- [x] VPN: Manual and managed virtual private network profiles.

## Notifications, Focus & Sound
- [ ] Notifications: Application alert styles (banners, alerts), notification badges, previews, screen sharing alerts, and scheduled summaries.
- [x] Sound: Output volume/device selection, input device/levels, alert sounds, UI sound effects, and spatial audio options. (Partially implemented)
- [ ] Focus: Do Not Disturb, Work, Personal, Sleep modes, Focus filters (per-app behavior), and schedule automations.
- [ ] Screen Time: Downtime, app limits, communication limits, content & privacy restrictions, and cross-device usage tracking.

## General & System Management
- [x] About: System specifications, OS version, storage breakdown, coverage/warranty.
- [ ] Software Update: OS upgrades, automatic update cadences.
- [x] Storage: Local drive breakdown, recommendations, optimized storage tools.
- [ ] Login Items & Extensions: Apps open at login, background processes/daemons, extensions, plugins.
- [x] Language & Region: System display language, regional formatting (dates, measurement units, currency), preferred languages.
- [x] Date & Time: 24-hour time, time zone (automatic or manual), network time servers.
- [ ] Sharing: File sharing, screen sharing, remote login (SSH), remote management, printer sharing, internet sharing.
- [ ] Transfer or Reset: Migration Assistant, Erase All Content and Settings.

## Appearance, Display & Desktop
- [x] Appearance: Light, Dark, or Auto theme, accent color, highlight color, sidebar icon size, auto-hiding scrollbars. (Partially implemented)
- [ ] Accessibility: Vision, Hearing, Motor, General.
- [ ] Control Center: Menu bar item toggles (Wi-Fi, Bluetooth, Battery percentage, Clock, Fast User Switching).
- [ ] Desktop & Dock: Dock size, magnification, screen position, minimize animation, automatically hide/show Dock, Window Manager configuration, workspaces, widget placement.
- [x] Displays: Resolution scaling, refresh rate, color profiles, arrangement. (Partially implemented)
- [ ] Wallpaper: System wallpapers, custom photo folders, shuffle frequency.
- [ ] Screen Saver: Visualizers, delays.

## Privacy, Security & Power
- [x] Battery / Energy Saver: Low Power Mode, battery health/capacity, power adapter sleep timers, wake for network access.
- [ ] Lock Screen: Display sleep delay, require password after display turns off, lock screen message, user switching display style.
- [ ] Login Password: User account login password.
- [x] Users & Groups: Local accounts, Guest user account, Fast User Switching, administrator privileges.
- [x] Privacy & Security: Location Services, Camera, Microphone, File encryption, Firewall, etc.

## Inputs & Peripherals
- [x] Keyboard: Key repeat rate, delay until repeat, keyboard shortcuts, input sources/layouts.
- [ ] Trackpad: Point & Click, Scroll & Zoom, More Gestures.
- [x] Mouse: Tracking speed, natural scrolling, primary/secondary click configuration.
- [ ] Game Controllers: Paired Bluetooth controllers, button remapping, calibration.
- [x] Printers & Scanners: Installed printing queues, default printer, default paper size, scanner drivers.

## Internet & Cloud Services
- [ ] Internet Accounts: Third-party account integrations.