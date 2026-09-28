# Store (Finick Apps) — in-progress export

Date: 2026-09-28. Everything below is staged-but-uncommitted in `~/Documents/Projects/finick` unless noted.

## 1. Finick Apps store (this session)

Goal: install from Flathub + nixpkgs, reproducible-first via Home Manager, `nix profile` only as fallback.

### New files
- `apps/store/Cargo.toml`, `apps/store/src/main.rs` (~850 lines) — Freya app: Discover / Installed / Updates tabs, detail view (nix meta + Flatpak permission Allow/Revoke), HM link + Apply bar. Styled to the `finick` skill (tile/tile_head/setting_row/page_head, pill search, short tokens, GAP).
- `libs/system/src/store_flatpak.rs` — search/install/remove/update(_single/_all), `metadata`, `permissions`, `set_permission` (flatpak override), `ensure_flathub_remote`.
- `libs/system/src/store_nix.rs` — `nix search --json` + `nqx` fast path, `eval_meta`/`package_details` (license, longDescription, version), `profile_install`/`profile_remove` (fallback only).
- `libs/system/src/store_hm.rs` — owns `~/.config/finick/store/{store.json,apps.nix}`; `render_nix`, `ensure_imported` (`nix-editor --arr-add imports <abs path>`, idempotent, backup + parse-check), `apply_home_manager` (`nh` then `home-manager`).
- `libs/system/src/store.rs` — `StoreEntry`/`StoreDetails`, `search_all`, `list_installed`, `install_nix_reproducible` (write nix → switch → profile fallback), `remove_nix_reproducible`.
- `services/finickd/src/store_worker.rs` — `StoreJob` enum + `run_job` reusing the above (daemon progress IPC not yet wired).

### Wiring (modified)
- `flake.nix` — `/bin/store`, `finick-store.desktop`, `apps.store`.
- `nix/home-manager.nix` — `programs.finick.store.{enable,extraPackages,flatpaks}` (dotted attrs via `getAttrFromPath`).
- `finickctl` — `-p apps '<search|list|install-nix|remove-nix|install-flatpak|remove-flatpak|link|apply>'` (+ `system` dep).
- `libs/system/src/lib.rs`, `services/finickd/src/main.rs` — module declarations.

### Key decisions
- Declarative by default; imperative fallback is marked and reconciled on next successful switch.
- Generated `apps.nix` imported by absolute path (relative `./modules/…` breaks on rebuild from elsewhere).
- System `.desktop` apps: `Preinstalled` chip, no Remove.

### Still open
- `responsive_view` for narrow windows (checklist item).
- finickd progress IPC for long ops (worker exists, no protocol yet).
- Per-package Nix option forms (`nix eval` on `programs.<pkg>` schema).
- Flatpak per-app update button calls `update_single` — verify against real flatpak.
- Verify search-field alignment visually (`cargo run -p store`).

## 2. Other in-progress changes in the tree (earlier, not this session)

- **Locker hardening** (`apps/locker`, `libs/system/src/lock.rs`, `services/finickd/src/main.rs`): `finick-locker.lock` lockfile in `$XDG_RUNTIME_DIR`, `locker_locked`/`locker_process_alive` helpers, finickd supervisor thread respawns a dead locker + stops overlay; locker finds `finick-locker` binary name, password field themed + autofocus, unlock releases single-instance + restarts overlay, multi-monitor support growing. Notes in `todo/locker.md`.
- **Overlay** (`apps/overlay/src/lib.rs`): `ensure_super_l_lock_binding` retries up to 60× until Hyprland answers (systemd race fix), factored `try_ensure_super_l_once`.
- **Skills**: `.agents/skills/finick/SKILL.md` added (UI consistency rules); `brandkit`, `imagegen-frontend-{web,mobile}` deleted.
- **Cargo.lock**: +17 (store/finickctl deps).

## 3. Verify
`cargo check -p store -p system -p finickd -p finickctl` clean; `cargo test -p system` 19 pass; `cargo fmt` applied.
