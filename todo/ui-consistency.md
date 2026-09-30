# UI Consistency (apps/ vs libs/ui/)

Updated: 2026-09-30.

Goal: every `apps/` view uses shared `libs/ui` helpers instead of
hand-rolled rects — one card, one row, one switch, one search, one
button, one chip. No visual change, just dedup.

## Files

- `libs/ui/src/components/{tile,row,switch,button,chip,sidebar,slider,segmented,select_row,list_item,page_head,topbar,context_menu,storage_bar,chart,layout}.rs` — canonical helpers.
- `apps/settings/src/{overview.rs,detail_pages.rs}` — canonical usage. Do not diverge.
- `apps/settings/src/pages/{bluetooth,network,language,printers,storage,about,date_time,privacy,shortcuts,default_apps}.rs` — most drift lives here (legacy `pages/*` predate shared helpers).
- `apps/{files,launcher,locker,store}/src/main.rs` + `apps/overlay/src/{panel,launcher,modal,clipboard_manager,screenshot,notification_panel,notification_popup,session,osd,lib}.rs` — second drift cluster.

### Key decisions

- Direct swaps first (P0): `pill_switch`, `primary/secondary_button`, `icon_button`, `setting_row`, `tile/tile_head`, `status_chip`, `sidebar_search`, `ctx_button`, `page_head`. No API changes.
- New shared components second (P1): `small_close_button`, `search_field`, `dialog_card`, `section_label`, `dot_status_row`, `empty_state`, `stat_block/hero_identity`, `notice_pill`. Add once in `libs/ui`, then migrate call sites.
- `quick_action_tile` needs a flex-width variant before `session.rs` / `screenshot.rs` can migrate (fixed `120x90` does not fit `flex:1 x 56-72`).
- `page_header` (28px, icon-less) retires in favor of `page_head` (21px + icon). One header hierarchy only.

## Done (2026-09-30, code changed)

- [x] Full `apps/` vs `libs/ui` audit: ~60 direct-replacement sites + 9 new-component candidates.
- [x] P0 switches: all raw `Switch::new` → `pill_switch` in bluetooth, network, language, printers, date_time, privacy.
- [x] P0 buttons: `Button::new` → `primary/secondary_button` in overlay/modal; Unlock rect → `primary_button_full` in locker; bordered icon rects → `icon_button` in files.
- [x] P0 rows+cards: manual rows → `setting_row`; manual cards → `tile()+tile_head()` across settings + overlay/panel.
- [x] P0 chips+bars+menus: badges → `status_chip`; bars → `input_level_pill`/`multi_segment_bar`; `MenuButton` → `ctx_button`; files toolbar → `top_bar_custom`; bare `Input` → `sidebar_search`/`search_field`.
- [x] P0 headers+labels: `page_header` → `page_head`; ALL-CAPS → `field_label`/`nav_group_label`/`section_label`, sentence case copy.
- [x] P1 new shared in `libs/ui`: `small_close_button`, `search_field`, `dialog_card`, `section_label`, `dot_status_row`, `action_tile_flex`, `empty_state`, `stat_block`/`hero_identity`, `notice_pill`.
- [x] P1 migrate: dialogs → `dialog_card`; closes → `small_close_button`; status rows → `dot_status_row`; session/screenshot → `action_tile_flex` (deleted `session_btn`); empty views → `empty_state`; stats/heroes → `stat_block`/`hero_identity`; legacy tokens (`bg_card`/`border_card`/`text_primary` etc) → short set (`panel`/`border`/`text`/`text_dim`).
- [x] Animations: `FadeSlideIn` on settings pages, overlay modals/session/screenshot/clipboard/notifications/launchers, files modals, locker card. `PanelShell` skipped (needs dismiss wiring).
- [x] Verify: `cargo check --workspace` passes (pre-existing warnings only); `cargo fmt --check` clean.

## Risks

- `icon_button` is `36x36/radius-8`, close pills are `28x28/pill` — direct swap changes size; ship `small_close_button` first instead of forcing the fit.
- `quick_action_tile` fixed size vs flex needs — same: add variant first.
- Locker uses a custom dark-glass theme; `lock_badge`/`primary_button` swaps there need visual check.
- Launcher duplicate (`apps/launcher` + `overlay/launcher`) doubles every fix — consolidate first (see `todo/launcher.md` P0).

## Open questions

1. Retire `page_header` outright or keep as deprecated alias?
2. `section_label`: uppercase + letter-spacing as a new helper, or fold into `field_label`?
3. Keep `apps/launcher` crate during migration or shim it first?

## Verify

`cargo check --workspace`; `cargo fmt --check`; live: settings every page, files toolbar/search/menus, launcher search+clear, overlay panel/modals/screenshot/session, store search — pixel-parity except 28px→shared close buttons and pink-switch→pill fixes.
