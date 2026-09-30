# Store (Finick Apps)

Updated: 2026-09-30. Work in `~/Projects/finick`, uncommitted.

Goal: install from Flathub + nixpkgs, reproducible-first via Home Manager, `nix profile` only as fallback.

## Files

- `apps/store/src/main.rs` — Freya app: Discover / Installed / Updates tabs, detail view (nix meta + options card + Flatpak permission Allow/Revoke), HM link + Apply bar. Styled to the `finick` skill (tile/tile_head/setting_row/page_head, pill search, short tokens, GAP). All 5 op sites go through one `spawn_store_job` helper with live notice lines.
- `libs/system/src/store_flatpak.rs` — search/install/remove/update(_single/_all), `metadata`, `permissions`, `set_permission` (flatpak override), `ensure_flathub_remote`. Search columns fixed (`parse_search_line`, unit-tested).
- `libs/system/src/store_nix.rs` — `nix search --json` + `nqx` fast path, `eval_meta`/`package_details` (fallback only).
- `libs/system/src/store_hm.rs` — owns `~/.config/finick/store/{store.json,apps.nix}`; `render_nix`, `ensure_imported` (idempotent, backup + parse-check), `apply_home_manager` (`nh` then `home-manager`); `StoreDecls` also carries `program_options` (dotted paths → `apps.nix` section).
- `libs/system/src/store_opts.rs` — per-package HM option schemas: `nix eval` on matched nixos-25.05 + HM release-25.05, JSON-cached under `~/.cache/finick/hm-options`; scalar kinds editable (bool/string/int/float/small enums), rest read-only; `nix_literal` renderer.
- `libs/system/src/store.rs` — `StoreEntry`/`StoreDetails`, `search_all`, `list_installed`, `install/remove_nix_reproducible`, plus the job protocol: `StoreJob` (+`UpdateFlatpak`), `StoreRequest`/`StoreResponse` (`Started`/`Log`/`Finished`) on the `finick-store` socket, `run_job` + `run_job_ipc_or_local` (local fallback when the daemon is unreachable).
- `services/finickd/src/store_worker.rs` — thin `handle_store_request` (blocking per-connection worker); socket served from `finickd/src/main.rs` alongside settings/notifications.
- `libs/system/src/auth.rs` — `authenticate_user`: dev backdoors → PAM stack (`finick-lock`, else `login`; libpam dlopened, no link dep) → legacy sudo/su/shadow chain. Used by the session locker; store benefits via daemon-side jobs running as the user.

### Wiring (modified)
- `flake.nix` — `/bin/store`, `finick-store.desktop`, `apps.store`. (Plus a pre-existing local path-fallback tweak, not ours.)
- `nix/home-manager.nix` — `programs.finick.store.{enable,extraPackages,flatpaks}` (dotted attrs via `getAttrFromPath`).
- `finickctl` — `-p apps '<search|list|install-nix|remove-nix|install-flatpak|remove-flatpak|link|apply>'` (+ `system` dep). Still drives ops directly, not via the daemon socket.
- `libs/system/src/lib.rs`, `services/finickd/src/main.rs` — module declarations.
- `devenv.nix` — `pkgs.linux-pam` (packages + `LD_LIBRARY_PATH`).

### Key decisions
- Declarative by default; imperative fallback is marked and reconciled on next successful switch.
- Generated `apps.nix` imported by absolute path (relative `./modules/…` breaks on rebuild from elsewhere).
- System `.desktop` apps: `Preinstalled` chip, no Remove.
- Narrow windows: root wrapped in `responsive_view(780)` (sidebar 232→180, padding shrink, same pattern as settings).
- Option values equal to the module default are pruned, keeping `apps.nix` clean.
- `Log` exists in the job protocol for future line-streaming; jobs report start/finish today.

## Done this round
- [x] `responsive_view` for narrow windows.
- [x] Flatpak search column mapping (+ `update_single`/`pending_updates` verified against live flatpak).
- [x] finickd progress IPC (protocol, daemon server, app helper, fallback; unit-tested incl. temp-socket roundtrip).
- [x] Per-package Nix option forms, first slice (detect → cached schema → card → write-back; unit-tested kinds/fixture/literals).

## Still open
- Visual verify (`cargo run -p store`) — manual; headless here.
- Option forms, later: fuzzy module matching (`firefox-esr` → `programs.firefox`), raise the 12-editable/6-readonly caps, TTL or release-keyed schema cache, `Log` line-streaming from workers, route `finickctl apps` through the daemon socket.

## Verify
`cargo check --workspace` clean; `cargo test -p system` 28 pass + `cargo test -p finickd` 4 pass + `cargo test -p session-lock` 4 pass (1 pre-existing env failure: `test_tools_check` needs the `wl-screenrec` binary); `cargo fmt --check` clean; clippy clean on touched crates (pre-existing `libs/ui` lints untouched).
