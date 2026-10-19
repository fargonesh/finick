# Launcher

`apps/launcher` (`launcher` binary, `SUPER+SPACE`). Empty query shows recents, never a blank list.

## Providers (in order)

- Calculator: `=` or `calc` prefix (e.g. `= 42*3`, Enter copies)
- Finick commands: `:` prefix
- Shell: `!` prefix — explicit "Run command: …" row, with failure feedback. Plain Enter text never executes as shell.
- Index results: apps, folders, files with kind chips (`App`, `Folder`, `File`, `Calc`, `Cmd`, `Exec`) plus path / size / date meta

No-result state tells you what to do next ("Prefix with `!` to run").

## Keys

`ArrowUp`/`ArrowDown` and `Ctrl+J`/`K` move selection, `Tab` completes the name, `Enter` launches, `Shift+Enter` reveals in folder. Selection clamps after every search; per-keystroke generation counters drop stale index responses.

## Toggle from scripts

`finickctl -p launcher -s toggle` kills the running launcher via `/tmp/finick-launcher.lock`, otherwise starts `launcher` (falling back to `hyprctl dispatch exec -- launcher`).
