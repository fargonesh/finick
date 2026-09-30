---
name: finick
description: Finick UI consistency rules for apps/ and libs/ui/. Use when touching settings pages, files, or any shared component to keep the polished look and avoid the regressions seen in shortcuts, firewall, locales and files views.
user-invocable: true
---

# Finick UI

Companion skills. Load them first when the task matches:
- `freya` for component shape, hooks, layout, state, events. All rules there apply.
- `anti-ai-slop-writing` for any user facing copy in headers, empty states, buttons, tooltips.

Canonical reference is `apps/settings/src/overview.rs` and `apps/settings/src/detail_pages.rs`. Regressions live in `apps/settings/src/pages/privacy.rs`, `apps/settings/src/pages/language.rs`, `apps/settings/src/pages/shortcuts.rs`, `apps/files/src/main.rs`.

## Reuse first (stops drift before it starts)

Before writing any `rect().on_press`, `Button::new`, `Switch::new`, `MenuButton::new`, `Input::new`, badge, bar, close `✕`, card, or row: grep `libs/ui/src/components/*.rs` for an existing helper. If one fits, use it. If two files need the same shape and no helper fits, add it to `libs/ui` once instead of copying the rect twice.

Full helper map: `tile`, `tile_head`, `field_label`, `tile_sub`, `setting_row`, `setting_row_locked`, `pill_switch`, `slider_row`, `segmented_control`, `status_chip`, `primary_button`, `secondary_button`, `danger_button`, `ghost_button`, `icon_button`, `focus_pill`, `quick_action_tile`, `sidebar_search`, `select_row`, `dropdown_select`, `list_item`, `list_item_with_subtitle`, `page_head`, `top_bar_custom`, `ctx_button`, `ctx_divider`, `multi_segment_bar`, `input_level_pill`, `nav_group_label`, `nav_item`.

Banned in `apps/`: raw `Switch::new`, raw `Button::new`, raw `MenuButton::new`, hand-rolled `28x28` close pills, hand-rolled badges/bars, bare `Input` search fields, manual `bg_card` cards, manual `bg_base` rows, `page_header()` legacy headers.

## Cards

Use shared helpers. Never hand roll.

Good:
`tile()` then `tile_head()` then `setting_row()` or `tile_sub()` or `field_label()`.

Bad:
`rect().width(fill).margin((0.,0.,16.,0.)).padding(16.).corner_radius(12.).background(t.bg_card).border(...fill(t.border_card))`

That duplicate is why firewall and locales drifted. `tile()` is `padding 18`, `radius RADIUS_LG (20)`, `background t.panel`, `border t.border`. Keep it.

Tokens: prefer `panel`, `panel_raised`, `border`, `text`, `text_dim`, `track`, `accent`. The `bg_card`, `border_card`, `text_primary` aliases resolve to the same values but split the codebase. Pick the short set.

Spacing: `GAP (14)` between cards and rows. Header margins come from `tile_head` and `page_head`, do not add extra.

## Headers

Good:
`page_head(ICON, "Title", "Description")` for pages.
`tile_head(Some(ICON), "Title", Some(action))` for cards.
`field_label()` for small grey labels.
`tile_sub()` for hints.

Bad:
`page_header()` big 28px title, manual `ALL CAPS` labels with `font_size 13 BOLD color text_secondary`, custom title rows. That is the firewall look.

Keep one hierarchy only.

## Switches and controls

Use `pill_switch(check, on_toggle)` in `libs/ui/src/components/switch.rs`. It is `36x21`, track `accent` or `track`, white thumb.

Never use raw `Switch::new()` in settings. It renders the default pink Freya switch and breaks the theme.

Other controls:
`segmented_control()` for Light/Dark/Auto, Small/Medium/Large.
`slider_row()` for brightness, volume.
`status_chip(text, is_on, on_press)` for Connected, Paired, INSTALLED, DEFAULT, kind badges. Never hand-roll `rect().padding().corner_radius(999/4)` badges.
`primary_button()`, `secondary_button()`, `danger_button()`, `ghost_button()`, `focus_pill()` for actions. Never `Button::new()` in overlays/modals, never accent `rect().on_press` (locker Unlock), never bordered icon `rect().on_press` (files toolbar). Do not wrap them in extra bordered rects.
`icon_button(icon, on_press)` for toolbar icons. Never hand-roll `rect().padding(8).corner_radius(8)` icon boxes.
`ctx_button()` / `ctx_divider()` for context menus. Never raw `MenuButton::new()`.
`input_level_pill()` / `multi_segment_bar()` for OSD/storage bars. Never hand-roll `rect().height(8).corner_radius(999)` meters.
`top_bar_custom()` for app top bars. Never hand-roll `rect().height(56)` chrome.
`list_item()` / `list_item_with_subtitle()` + `status_chip` for launcher result rows.
`select_row()` / `dropdown_select()` for static-value pickers (First Day of Week).
`quick_action_tile()` for session/screenshot tiles; add a flex-width variant rather than copying `session_btn` helpers.

## Inputs and the border in border rule

Freya `Input` draws its own border. So only one border per field.

Correct pill search pattern, see `sidebar_search()` in `libs/ui/src/components/sidebar.rs`:
parent `rect` with `corner_radius RADIUS_PILL`, `background panel`, `border border`, plus inner `rect width flex` with `content Flex`, plus `Input` with `background TRANSPARENT`, `border_fill TRANSPARENT`, `focus_background TRANSPARENT`, `focus_border_fill TRANSPARENT`.

Failings found:
- `shortcuts.rs:57` raw `Input width flex` with no wrapper. Single border but wrong style, mismatched with the rest.
- `language.rs:723` wrapper is right but inner `Input` has no `width fill`. Parent expands, input does not, text clips to `Search installed lo`.
- `files/main.rs:426` `Input width px 140` fixed. Too narrow, cramped toolbar. Use `fill` or `flex`.
- `files/main.rs:404` path field with `.flat()` inside a bordered parent is correct. Copy that.

Rule: styled parent means transparent Input plus `width Size::fill()`. Unstyled row means use the pill search pattern instead of a bare Input. Never fixed `px` widths for search. Always `content(Content::Flex)` on any `rect` with `flex` children per `AGENTS.md`.

## Lists and grids

Use `setting_row(name, meta, has_top_border, control)` with `content Flex`. It handles title plus dim meta plus right control.

Use `responsive_view(breakpoint, |compact|)` and `responsive_stack(compact, [...])` from `layout.rs`. Overview uses `1000.0`, detail pages use `720.0` to `1000.0`.

Files grid drift: fixed `120x140` cells, `wrap_spacing 6`, icons `24/32/48`. Result is tiny icons in a loose grid. Keep cells tight, icons larger, spacing from `GAP`. Avoid fixed `220 / 56 / 280` chrome where flex works.

Properties drawer drift: `Cut/Copy` and `Rename/Delete` each wrapped in `rect width flex` around a non expanding button. Buttons end up uneven. If two across, make the button itself fill or use one shared row component.

## Nesting

No card inside a card unless it is an inset readout. Shortcuts add form (`panel_raised` plus border plus radius 12 inside `tile`) reads heavy. Prefer flat `field_label` plus `Input` rows with `spacing 8`.

Status inset like the firewall `bg_base` box is allowed, but keep `radius 8`, single subtle border, no extra margins.

## Dialogs, empty states, labels

Modal cards (`overlay/modal.rs`, `osd.rs`, `session.rs`, centered `380-520px`): use `tile()` today; extract `dialog_card()` when touching them twice. Never duplicate `width(400).padding(20).corner_radius(16)` per dialog.
Empty states (`No clipboard items`, `No notifications`, launcher hints): use `tile_sub()`, extract `empty_state(icon, title, subtitle)` on repeat.
Section labels: use `field_label()` / `nav_group_label()`, never manual `11-13 BOLD text_muted ALL CAPS`.
Status dot rows (wifi/BT `●` + Connected): use `setting_row()`, extract `dot_status_row()` on repeat.
Spec triplets / hero rows (Used/Available/Total, avatar + name): extract `stat_block()` / `hero_identity()` rather than copying label/value rects.

## Motion (helps the user, never shows off)

Every animation must answer one question: where did this come from, did my action register, or what do I read first. If it answers none of those, delete it.

Use the primitives in `libs/ui/src/motion/`. Never hand-roll `use_animation` in `apps/`.

Helper map: `FadeSlideIn` (panel and card enter: opacity plus 8px rise, optional `scale_from`), `FadeIn` (opacity only), `PanelShell::new(dismiss)` (enter plus deferred exit, fires `on_dismiss` after the fade so `close_window` never pops), `AnimatedPage::new(route_key)` plus `PageDirection` (settings subpage enter), `PressScale` (press feedback wrapper), `stagger_delay(index, step, max)` for list enter, tokens `DUR_FAST 120` / `DUR_STANDARD 170` / `DUR_EMPHASIS 240`.

Good: panel opens with `PanelShell`, 170ms Expo-Out, opacity plus rise plus 0.97 scale. Settings subpage enters with `AnimatedPage`, Forward slides from +24px, Back slides from -24px, 240ms Cubic-Out.
Bad: 500ms Elastic panel bounce, per-row stagger on a 50-row scroll list, animating `width`/`height` so the whole layout reflows mid-transition.

Rules:
- One motion per transition. Enter is opacity plus one axis of movement. Never opacity plus slide plus scale plus color at once.
- Prefer `opacity`, `offset_x`/`offset_y`, `scale`. They composite. `width`/`height` reflow layout and jank on large trees.
- Keep hover instant. Hover is a state query the pointer answers every frame, a fade on hover always lags the hand. Animate selection and activation, not hover.
- Press feedback goes on the way down fast (`PressScale`, 120ms) or not at all. Never delay the press response to wait for an animation.
- Never block input on animation. No modal waits, no `sleep` before `close_window`. `PanelShell` defers close by ~170ms after the user already acted, it never holds the action hostage.
- Stagger is for short entering lists only (5 to 8 items, 20 to 30ms step, capped near 120ms total). Never stagger on scroll, filter keystrokes, or anything that re-runs per frame.
- No loops, no Bounce/Elastic/Back for standard UI. Reserve them for playful one-shots an adult could defend in review.
- Exit is the same curve reversed, same duration or shorter. If there is no exit path (`PanelShell` dismiss, `AnimatedPage` settle), there is no enter animation either.
- Reduced motion: every motion component takes `.reduced(true)`. When touching a settings toggle for motion sensitivity, thread it through as `reduced` instead of adding a second code path.
- Animation hooks live in `libs/ui` components (`SwitchThumb`, `NavBody` own theirs), never in app loops, conditionals, or event handlers. If an `apps/` file needs a new animated shape, add the component to `libs/ui` once.

Failings to avoid:
- Animating the thumb but also crossfading the track plus sliding the row. Pick the thumb.
- Fading a whole settings page on every keystroke in its search field. Enter animation keys off `route_key`, not off filter text.
- Holding a panel open an extra 300ms to finish a flourish. The user already dismissed it, the window owes them speed.

## Checklist before ship

1. Did I use `tile`, `tile_head`, `setting_row`, `page_head` instead of manual rects.
2. Any raw `Switch`, `Button`, `MenuButton`, or raw `Input` left. Replace with `pill_switch`, shared buttons, `ctx_button`, or pill search.
3. Any hand-rolled close `✕`, badge, bar, top bar, result row. Replace with `icon_button`, `status_chip`, `input_level_pill`/`multi_segment_bar`, `top_bar_custom`, `list_item`.
4. One border per input. Transparent Input in styled parents, full width.
5. Tokens from the short set, `GAP` spacing, `RADIUS_LG/MD/SM/PILL`.
6. No fixed search widths, no uneven button wraps, responsive helpers where Overview uses them.
7. If I copied a rect twice, did I move it to `libs/ui` instead.
8. Copy passes `anti-ai-slop-writing`, components pass `freya`.
9. Every animation uses a `motion/` primitive with a token duration, answers where/confirmed/first-read, and has a matching exit or settle path.
10. No hover fades, no layout-size animation, no stagger on scroll or keystrokes, no input blocked on motion.
