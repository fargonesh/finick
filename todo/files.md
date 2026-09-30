# Files (File Browser + Index)

Updated: 2026-09-30.

Goal: daily-drivable file browser — browse fast, find fast, never lose data
on a misclick. `apps/files` thin-client over `services/index`, local `read_dir`
only as fallback.

## Files

- `apps/files/src/main.rs` — everything (~1415 lines, single file): `Item` /
  `ViewMode` / `IconSize`, `fetch_via_index` + `load_dir` + `perform_search`,
  sidebar (Home/Documents/Downloads/Pictures + `places.json` pinned + Add
  Place), top bar (sidebar toggle, up-one-level, path `Input`, search `Input`,
  S/M/L toggle, grid/list toggle, New Folder), grid (`120x140`, `ImageViewer`
  per image) / list rows, single-select + fake rubber-band, item context menu
  (Open/Cut/Copy/Paste/Rename/Delete), 280px properties drawer, Add Place +
  Rename modals, `Ctrl+C/X/V` + `F2` + `Delete` shortcuts, `do_paste` +
  `copy_dir_recursive` (blocking `std::fs` in handlers).
- `services/index/src/{ty.rs,main.rs,lib.rs}` — backend: `Request::Search/ListDir`,
  `SearchResult { name, path, icon, is_desktop, is_executable, is_dir, size,
  modified }`. LIKE search + `$PATH` fallback.
- `apps/files/Cargo.toml` — `freya(all)`, `tokio(full)`, `image`, `index`,
  `config`, `ui`, `serde_json`.

### Wiring (as-found)

- No flake/HM/Desktop-entry gaps checked this round — verify `flake.nix`
  ships `/bin/files` + `finick-files.desktop` like launcher/store.
- Pinned places persist to `~/.config/finick/places.json` (app-local, not
  via `finick_root()` / config lib — confirm canonical path).
- Open via `xdg-open` spawn; no Open-With / Terminal=true handling.

## Key decisions (proposed)

- Stay thin-client: sorting/filtering in UI, listing/searching in index.
  No local walk except the existing fallback.
- Destructive ops go to trash first (deletion without confirm is the
  current top data-loss risk), with undo + empty-trash.
- Long ops (copy/move/delete/search) leave event handlers: background job
  with progress + cancel + error toast, not blocking `std::fs`.
- Search is a mode, not a silent replace: scoped to current dir by default,
  with visible scope + clear + Esc-to-exit.

## Done (already works)

- [x] Browse via index with `read_dir` fallback, folders-first sort.
- [x] Grid/List + S/M/L icons, image thumbnails in grid + drawer.
- [x] Sidebar places + pin/unpin + Add Place modal, collapsible.
- [x] Path input, global search input, New Folder (unique-name).
- [x] Item menu + drawer: Open, Cut/Copy/Paste, Rename (modal + F2), Delete
  (key + button), Pin folder, Paste Here.
- [x] Single-select, double-click open, right-click menus, empty-space Paste.

## Still open (phases)

| Phase | Work | Est |
| ----- | ---- | --- |
| P0 safety | Trash instead of `remove_*` permanent delete + confirm modal + undo; overwrite confirm in `do_paste`; error toasts (failed rename/copy/delete/paste surface nothing today); fix `format_size` (0 → empty, <1KB → `0 KB`, no bytes/B); search-mode indicator + clear/Esc back to dir (today results silently replace browsing) | 2–3 days |
| P0 correctness | Cancel stale loads (generation counter — rapid nav races today); loading / empty / error states (today `set(Vec::new)` flickers empty); move `copy_dir_recursive` + paste off UI thread; `places.json` via canonical config root | 2–3 days |
| P1 navigation | Breadcrumbs (clickable segments, today raw path string only); history back/forward + refresh + Up (today up-only, no forward); sort control (name/size/modified, dirs-first toggle); show-hidden toggle; status bar (N items, selected size, free space); full keyboard (arrows/Enter/Ctrl+A/Esc) | 3–4 days |
| P1 selection | Real multi-select (Ctrl/Shift + rubber-band selects — today drag box is visual only, `on_pointer_enter` sets a single item); clipboard holds N paths + cut badge; empty-space menu: New Folder/File, Paste, Refresh, View, Show hidden | 2–3 days |
| P2 ops | New File + Duplicate; properties that matter (modified, type, permissions — index already returns `modified`, UI ignores it); Open-With + Show in folder; rename validation (empty/exists); progress + cancel for copy/move/delete | 3–5 days |
| P3 polish/perf | Cached/small thumbnails (today full `ImageViewer` per grid cell); text/pdf preview; file watcher auto-refresh (notify); sidebar Devices/Trash/Recent (+ existence checks — fixed entries assume `~/Documents` etc. exist); `finick` skill pass (see below); empty-trash + restore | ~1 week |

### `finick`-skill alignment (from audit)

- Search `Input width px 140` too narrow — use fill/flex pill pattern
  (`transparent` Input + `width fill` + `content Flex` parent).
- Grid `120x140` + `wrap_spacing 6` + `24/32/48` icons reads loose — tighter
  cells, larger icons, `GAP` spacing.
- Chrome `220 / 56 / 280` fixed — flex + `responsive_view` like settings
  overview (`1000.0`) / store (`780`).
- Drawer `rect width flex` around non-expanding Cut/Copy/Rename/Delete
  buttons → uneven halves — buttons fill or shared row component.
- Tokens: short set (`panel`, `border`, `text`, `text_dim`, `accent`, `GAP`,
  `RADIUS_*`) over `bg_card/border_card/text_primary` drift.
- Copy pass per `anti-ai-slop-writing` for empty/error states.

## Risks

- Permanent delete + silent overwrite is one misclick from data loss
  (mitigate: P0 trash + confirm first).
- Blocking fs in handlers freezes the window on big copies (mitigate: P0
  background jobs).
- Full-image thumbnails grind large dirs (mitigate: P3 cached thumbs).
- Index LIKE ranking + unscoped global search confuses browsing
  (mitigate: scoped search + visible mode).

## Open questions

1. Trash backend: `trash` crate vs own `~/.local/share/Trash`?
2. File watching: `notify` in app vs push from index service?
3. Tabs/split wanted at all, or single-pane + launcher is enough?
4. Keep `places.json` app-local or move under shared config root?

## Verify

`cargo check -p files`; live: browse/home→docs, search + clear, New
Folder/File, rename, copy/paste (overwrite asks), delete → trash + undo +
restore, multi-select + rubber-band, hidden toggle, sort, breadcrumbs +
back/forward/refresh, reload survives restart (pins), `cargo fmt --check`.
