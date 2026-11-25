# Finick Logical System

This doc defines how state, mutations, async work, and errors should flow across `apps/`. The review that produced it found the same failure in several places: polished controls that implied system truth the backend didn't have, optimistic writes with no rollback, and ignored command failures.

## Ownership

Four layers, each with one job.

1. Local UI state (`use_state` in the component): filter text, selected row, open/closed, arm/confirm for destructive actions, hover highlighting. It never claims system truth. Hover lives in the shared `libs/ui` component that owns it and never touches the store or IPC.
2. `SettingsStore` (`apps/settings/src/state.rs`): reactive mirror of daemon truth for wifi power, bluetooth power, volume, brightness, focus mode, lock states, and similar keys. Components read from it; they don't own the value.
3. Daemon truth (`finickd` over IPC, `SettingKey`/`SettingValue`): the authority. `set_and_apply` either accepts the write or denies it with a reason.
4. System backends (`libs/system`, `HyprlandBackend`, `nmcli`, `bluetoothctl`, `wpctl`, `lpstat`): the commands and files that actually change the machine.

Data moves one way: backend to daemon to store to UI. Writes move the other way: UI intent to store intent to daemon to backend, with confirmation coming back down.

Don't let a component write directly to a backend and also write to the store. That split-brain is how the firewall toggle ended up flipping local state while `ufw` never changed.

## Operation States

Every mutation uses the same six states:

```
Idle, Loading, Applying, Success, Unavailable, Failed, Managed
```

Managed means NixOS or policy locks the key. Show "Managed by NixOS" or "Locked by policy" with the lock badge; don't just dim the control and hope users get it.

Loading covers first fetch (wifi scan, printer list, locale list). Applying covers the window between user intent and daemon confirmation. Failed always pairs with what happened and what to do next: Retry, Reveal, Copy log. Unavailable means the hardware or service isn't there (no battery, no CUPS, no Bluetooth adapter). It gets an honest empty state, never a fake control.

`SettingsStore::set()` used to apply optimistically and log IPC failures to stderr. That pattern is retired. Optimistic UI is fine, but denial or IPC error must roll the local value back to daemon truth and surface `last_error` inline. Silent divergence is a bug.

## Mutation Flow

The shape, using settings as the reference:

```rust
// 1. Capture previous truth before you touch anything
let prev = store.volume.read().clone();

// 2. Apply intent locally so UI feels instant
store.apply_to_local_state(&SettingKey::AudioVolume, &val.into());

// 3. Send through IPC off the UI thread
spawn(async move {
    match set_and_apply(SETTINGS_SOCKET_NAME, key, val).await {
        Ok(Ok(())) => { /* confirm, clear error */ }
        Ok(Err(denial)) => {
            // 4. Roll back, show recoverable error
            store.apply_to_local_state(&key, &prev.into());
            store.last_error.set(Some(denial.to_string()));
        }
        Err(e) => {
            store.apply_to_local_state(&key, &prev.into());
            store.last_error.set(Some(format!("Couldn't reach settings daemon: {e}")));
        }
    }
});
```

Notes from code that hurt:

- `State` is not `Send`, so don't move it into `std::thread::spawn`. Use Freya `spawn` plus `spawn_blocking` for the CLI call, or pass values across a channel and set state back on the UI task. Settings hit this directly when rollback moved into a raw thread.
- Locked keys reject immediately with a reason. No partial write, no spinner that never resolves.
- Debounce sliders (volume, brightness, gaps). Don't fire `hyprctl` or `wpctl` per pixel. Apply on settle or at a short interval.
- Preview-only controls must say so in the UI ("Preview only, does not enforce policy"). Privacy shipped toggles that read like enforcement and had to be relabeled after review.

## Async Pattern

Components stay synchronous. `render` never awaits.

- One-shot fetch on mount: `use_hook` plus `spawn`, with blocking CLI work inside `spawn_blocking` (`nmcli dev wifi`, `bluetoothctl devices`, `lpstat -p`, `locale -a`). Settings overview does this for network info and paired Bluetooth devices.
- Streaming updates: channel plus `spawn` reader that calls `set_if_modified`. Prefer `set_if_modified` over `set` when polling (panel polls wifi/wired/power every 8 seconds) so you don't wake subscribers on identical values.
- Search with generation counters: launcher increments `SEARCH_GEN` per keystroke and drops stale results. Without the counter, slow index responses overwrite newer queries.
- Polling loops need a stop condition or they outlive the window. Tie them to mount lifetime; don't spawn a global loop per render.

Blocking the render thread with `Command::output()` freezes the window. 2-second poll loops without change detection spam renders. Both showed up in files (`dir_sig` watch) and panel polling before cleanup.

## Commands and Failures

Never ignore a child process result.

Bad: `let _ = Command::new("xdg-open").arg(&p).spawn();` Files did this in seven places, so broken associations failed silently.

Good: return bool or a result, surface `notice_pill("Open failed", Error)` with Retry or Reveal in folder.

Launcher's old fallback executed any Enter text through `hyprctl dispatch exec`. That was unsafe and opaque, so it now requires an explicit `!` prefix with a visible "Run command:" row and failure feedback. Copy that rule: destructive or ambient-execution actions need an explicit prefix or a confirmation, plus a visible result.

Store's old error path ended with "Full log in terminal as STORE_ERROR_JSON." That put recovery outside the app. The replacement keeps the Details toggle plus Copy log and Save log with in-UI feedback, and Discover shows Retry search when the notice is an error. Keep recovery inside the window.

`strip_ansi`, `clean_line`, `parse_error_summary` in store exist for a reason: raw Nix and build logs run thousands of lines. Show one honest summary line, keep 30 detail lines max behind Details, keep the full log one click away.

## Destructive Actions

Shutdown, restart, logout, package remove, file delete, network disconnect, and firewall changes share one flow:

1. First press arms. Button turns red to "Sure?" or a dialog opens with name, source, and timing.
2. Second press executes, or Cancel dismisses.
3. Arm reverts after about 5 seconds so a stray click doesn't linger.

Panel power actions use in-place arm plus a `dialog_card` with `danger_button` confirm. Store remove uses a modal overlay (`dialog_card(420)` plus scrim) with package name, source and short id, and a timing note ("Removes now" or "Removes the declaration, run Apply to rebuild"). Files delete uses pending paths plus a confirm dialog.

Nothing destructive executes on a single incidental click.

Undo where it exists: files keeps trashed items with an Undo chip. Prefer undo for reversible deletes; require confirm for irreversible or system-wide changes.

## Navigation and Selection

Settings root owns `current_route: Route`. Sidebar writes it, top bar Back clears to Overview, `Escape` clears search first then backs out of subpages. Keyboard order must match visual order. The old handler indexed filtered routes including About in the middle while the UI rendered About at the bottom, so focus jumped. The fix builds `visible_routes` (filtered minus About, About last) and wraps both ArrowUp/Down and Tab around it, with an accent-border ring on the focused item.

Launcher owns `query`, `results`, `selected`. ArrowUp/Down and Ctrl+J/K move selection, Tab completes the name, Enter launches, Shift+Enter reveals in folder. Selection index clamps after each search; stale indexes caused panics before.

Files owns `current_path`, `history`, `hist_idx`, `selected_paths`, `anchor_path`. Double click (450ms window) opens, single click selects with Ctrl/Shift range logic. History truncates forward entries on new navigation. Keep that invariant or Back/Forward lies.

Back always means up one level or back one view, never close the window unless you're at the root. Forward never invents a path.

## Search and Providers

Launcher separates providers explicitly: calculator (`=` or `calc`), finick commands (`:`), shell (`!`), then index results. Each row shows kind (`Calc`, `Cmd`, `App`, `Folder`, `Exec`, `File`) via `status_chip` plus path and size/date meta.

Rules:

- Empty query shows recents, never a blank list.
- No-result state says what to do next ("Prefix with ! to run", "Press Enter to run as command" only when a command prefix is present).
- Provider rows state their effect before execution ("Run command: ...", "= 42, Enter copies").
- Secondary actions (Reveal, Copy) appear in row meta (`Enter Open, Shift+Enter Reveal`), not only in footer text.

Files search states whether it searches the current folder or all locations. Settings filter matches title plus keywords; every route carries keywords for this reason.

## Fetch, Cache, Refresh

- Show cached or fast initial data immediately (`fast_initial_language_info`), then replace with live fetch. Don't block first paint on `locale -a`.
- Refresh buttons re-run the same fetch path as mount, not a copy. Language Refresh and firewall Refresh both reuse their mount fetch.
- Polling intervals: clock 1s, panel live state 8s, files dir signature 2s. Faster than that needs a reason.
- Empty means empty: "No printers found", "No networks found", "Empty folder". Don't render mock hardware when nothing is there. Printers once fell back to mock entries (`HP Laser Pro`) when no hardware existed; that fallback is gone.

## Copy and Labels

Labels must match behavior. "Application sandboxing" with an in-memory toggle overstated enforcement. "Enforce portal restrictions" had to become "Preview only, does not enforce policy." When the backend can't do it yet, the UI says Preview only, Managed, or Unavailable. It never implies control it lacks.

Buttons use verbs that match the code path: Save writes a draft value, Apply rebuilds, Retry re-runs the same job, Reveal opens the folder, Copy log copies. Don't label a declarative edit "Install" when it only stages config.

## Testing and Verification

No app-level test files existed at review time. `cargo check --workspace` passed with warnings that pointed at real unfinished work: unused Wi-Fi/BT modal state, dead panel sizing helpers, unused settings pages. Treat warnings as unfinished behavior.

Add tests where logic lives:

- Command parsing (`try_eval_expr`, provider prefix routing)
- Setting rollback (denial restores previous value, `last_error` set)
- Destructive confirmation (single press arms, second executes, timeout reverts)
- Navigation order (keyboard index matches visual order, Escape backs out)

Verify with `cargo check -p settings -p files -p overlay -p launcher -p store`. Keep each app warning-clean. A warning that stays is a decision, not noise.
