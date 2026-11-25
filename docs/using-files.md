# Files

`apps/files`: folders, favorites, previews, and search that keeps up while you type.

## Navigation and selection

- `current_path` with `history` / `hist_idx` / `selected_paths` / `anchor_path`.
- Single click selects (`Ctrl` toggles, `Shift` extends from the anchor). Double click within ~450ms opens.
- Back/Forward walk `history`; new navigation truncates forward entries.
- Back always means up one level, never closes the window at root.

## Search

The search field states its scope: current folder vs all locations. Index-backed results come from `services/index`.

## Destructive actions

Delete moves to trash with an Undo chip where reversible; irreversible deletes arm first ("Sure?") and revert after ~5s. Failures surface a `notice_pill` with Retry / Reveal in folder — never a silent no-op.

## Layout notes

Path and search fields use `flex`/`fill`, never fixed pixel widths. Grid cells use `flex` with `max_width`, the sidebar uses `flex(0.6)` capped at 260px so 640px windows survive. Directory polling runs on a ~2s signature check with change detection, not a blind re-render.
