# Launcher (Spotlight / Rofi-style)

Updated: 2026-09-30. Work in `~/Projects/finick`, uncommitted.

Goal: `SUPER+Space` spotlight overlay — fuzzy apps/files/execs, calculator,
commands, recents. Single instance, instant open, Esc dismisses.

## Files

- `apps/overlay/src/launcher.rs` — canonical UI: `launcher_app()` +
  `launcher_window_config()` + `launch_item()` + `search()`. Spotlight card
  (640x460, dim backdrop, search pill, result list, footer hints).
- `apps/overlay/src/bin/launcher.rs` — thin bin (`overlay:launcher`).
- `apps/launcher/src/main.rs` — legacy duplicate of the same UI. Kept for
  `devenv` (`-p launcher`) until consolidation. Do not diverge.
- `services/index/src/{lib.rs,ty.rs,main.rs}` — backend: sqlite `files` table,
  `Request::Search/ListDir`, `SearchResult { name, path, icon, is_desktop,
  is_executable, is_dir }`. LIKE search + `$PATH` 1-depth fallback.
- `finickctl/src/main.rs` — `finickctl launcher`: tries `launcher` in PATH,
  falls back to `hyprctl dispatch exec launcher`.

### Wiring (modified this round)
- `flake.nix` — `/bin/launcher` (`-p overlay --bin launcher`),
  `finick-launcher.desktop`, `apps.launcher`.
- `nix/home-manager.nix` — `SUPER SPACE exec launcher`, float/pin/noanim
  window rules for `class:^(launcher)$`.
- `devenv.nix` — `launcher.exec` already `cargo run -p launcher`; leave until
  consolidation (see P0).

### Key decisions
- One canonical UI in `overlay::launcher`. `apps/launcher` retires to a
  re-export shim in P0, not a second implementation.
- Search stays thin-client over `index` IPC. No local walk in the launcher.
- Launch via `hyprctl dispatch exec -- <cmd>` for desktop Exec / `$PATH`
  binaries, `xdg-open` for files/dirs. Exit-after-launch (`process::exit(0)`).
- Bare query + Enter = run-as-command fallback (spotlight behavior).
- Tokens from the `finick` skill short set (`panel`, `bg`, `border`, `accent`);
  result rows 56px, icon tile 36px, pill footer chips.

## Done this round
- [x] Audit: two copies found, flake + HM gaps identified.
- [x] `todo/launcher.md` scaffold.
- [x] flake packaging + HM bind + window rules.

## Still open (phases)

| Phase | Work | Est |
| ----- | ---- | --- |
| P0 consolidate | `apps/launcher` → shim over `overlay::launcher` (or delete); fix `flake.nix` `topbar` cargoArgs bug (`--bin overlay` twice, second should be `--bin topbar`); `devenv.nix` launcher → `-p overlay --bin launcher`; `cargo check` clean | 0.5 day |
| P1 ranking | index-side fuzzy score (subsequence + prefix boost, desktop-app first), debounce 80-120ms in UI, cancel stale searches (generation counter), empty-query recents | 2–3 days |
| P2 providers | calculator (`= 2+2`), finick commands (`:lock`, `:settings`, `:files`), clipboard history, emoji; provider trait with prefix dispatch | 3–4 days |
| P3 launch correctness | full `.desktop` Exec parsing (`%F/%U` args, `Terminal=true`, `TryExec`, `NoDisplay` filter), single-instance + toggle on re-invoke (SUPER+Space while open closes), focus-steal + Hyprland layer rules | 2–3 days |
| P4 polish | app icons from index (`icon` png path), file previews (size/modified), keyboard (Ctrl+J/K, Tab complete, Shift+Enter actions), usage-count recents in sqlite, `finickctl launcher --toggle` via daemon socket | ~1 week |

## Risks
- Two launcher copies drifting (mitigate: P0 shim immediately).
- Index LIKE ranking is weak for spotlight feel (mitigate: P1 scoring).
- `process::exit(0)` mid-Freya skips cleanup; acceptable for a launcher but
  single-instance toggle needs a lockfile/socket instead.
- `.desktop` Exec sanitizing is naive (`split_whitespace` breaks quoting).

## Open questions
1. Keep `apps/launcher` crate at all, or ship only `overlay:launcher`?
2. Calculator/eval crate choice (`meval` vs `fasteval`) — sandbox needs?
3. Recents store: new sqlite table in index vs flat file under `finick_root()`?

## Verify
`cargo check -p overlay` clean; `nix flake check`; live: `SUPER+Space` opens,
type filters, Enter launches, Esc closes, re-invoke toggles (after P3).
