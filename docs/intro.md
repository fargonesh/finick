# Introduction

Finick is a desktop shell for Hyprland on NixOS: settings, files, launcher search, top bar, app shop, and lock screen that share one theme and one backend story.

- `apps/settings` — 23 settings pages with search
- `apps/files` — file manager
- `apps/launcher` — `SUPER+SPACE` search (`launcher` binary)
- `apps/overlay` — `overlay` binary: top bar panel, quick settings, clipboard, screenshots, notifications, OSD, session dialog
- `apps/store` — app shop (nix + flatpak desired state)
- `apps/locker` — lock screen fallback (`finick-lock` is the session-lock client)
- `services/finickd` — settings daemon, the authority for `SettingKey`/`SettingValue`
- `services/index` — file indexer behind launcher and files search
- `finickctl` — CLI for index, accent, border, settings, notify, modal, launcher, apps

Data flows one way: system backends (`libs/system`) → `finickd` → `SettingsStore` → UI. Writes go the other way as intents and come back as confirmations or denials with rollback. See [Architecture](./architecture.md) and [Logical system](./logical_system.md).
