# Finick Design System

This doc defines how Finick apps should look and feel. It codifies what `libs/ui` already provides and stops the drift that produced the firewall, locales, shortcuts, and files regressions.

Canonical examples live in `apps/settings/src/overview.rs` and `apps/settings/src/detail_pages.rs`. When two files need the same shape and no helper fits, add it to `libs/ui` once instead of copying a `rect` twice.

## Tokens

All values come from `libs/ui/src/theme.rs`. Don't invent new ones in `apps/`.

Colors, short names first:

```
bg, panel, panel_raised, border, text, text_dim, track, accent
```

`bg_card`, `border_card`, `text_primary` resolve to the same values but split the codebase. Pick the short set and stick with it.

Radii:

```
RADIUS_LG 20.0   cards (tile)
RADIUS_MD 12.0   rows, inputs, result rows
RADIUS_SM 8.0    small insets, thumbs
RADIUS_PILL 999.0  pills, search, switches, close buttons
```

Spacing: `GAP` is 14.0. Use it between cards and between rows. Header margins come from `tile_head` and `page_head`, so don't add extra.

Type scale used across settings and overlay:

- Page title: 15 Bold (inside `page_head`)
- Card title: 13 SemiBold (inside `tile_head`)
- Row title: 13 Medium/SemiBold
- Row meta and hints: 12 `text_dim` (`tile_sub`)
- Micro labels: 11 Bold `text_muted`, only for secondary group headers like PLACES or DEVICES
- Keep 11px for secondary meta only. Primary info stays at 12px or larger or it falls apart at 150 percent scale.

## Cards

Never hand roll a card.

Good:

```rust
tile()
    .child(tile_head(Some(WIFI), "Wi-Fi", Some(action)))
    .child(setting_row("Connected".into(), Some("SSID".into()), false, control))
    .child(tile_sub("Wi-Fi is off"))
```

Bad: `rect().width(fill).margin((0.,0.,16.,0.)).padding(16.).corner_radius(12.).background(t.bg_card).border(...)`. That duplicate is why firewall and locales drifted. `tile()` is padding 18, radius `RADIUS_LG`, background `panel`, border `border`. Keep it.

Rules that kept biting us:

- One card, one topic.
- No card inside a card unless it's an inset readout (status box, log box). Shortcuts once nested a full `panel_raised` form inside `tile` and it read heavy. Prefer flat `field_label` plus `Input` rows with spacing 8.
- Status insets (firewall box, error detail box) use radius 8, one subtle border, no extra margins.

## Headers

One hierarchy only.

- Pages: `page_head(ICON, "Title", "Description")`
- Cards: `tile_head(Some(ICON), "Title", Some(action))` or `tile_head(None, "Title", None::<String>)`
- Small grey labels: `field_label()`
- Hints and empty copy: `tile_sub()`
- Sidebar groups: `nav_group_label()` plus `nav_item()`

Don't use `page_header()` legacy headers, big 28px titles, manual ALL CAPS labels, or custom title rows. That was the firewall look.

`tile_head` already handles spacing. Don't add margins around it.

## Controls

Switches: `pill_switch(checked, on_toggle)` from `libs/ui/src/components/switch.rs`. It's 36x21, track `accent` or `track`, white thumb. Hover is instant: track goes to `accent_blue_hover` when on, `border` when off. Raw `Switch::new()` renders the default pink Freya switch and breaks the theme, so it's banned in `apps/`.

Hover is a state swap, not an animation. Every shared button, switch, chip, and segment tracks hover internally with `use_state` plus `on_pointer_enter`/`on_pointer_leave` and swaps `background` on the next frame. No fade, no delay, per the Motion rules. Keyboard focus keeps its own ring (`accent` border) and hover never removes it. Hover colors per control:

- `primary_button`: `primary_accent` to `accent_blue_hover`
- `secondary_button`, `icon_button`: `bg_card` to `bg_hover`
- `danger_button`: `accent_red` to a lighter red
- `ghost_button`, `tool_button`, `small_close_button`, `focus_pill`: `panel_raised` to `track`
- `segmented_control` inactive segment: transparent to `bg_hover`
- `status_chip` with `on_press`: `track` to `panel_raised` when off, `bg_active` to `bg_hover` when on
- mute button in `slider_row_with_mute`: `panel_raised` to `track` when unmuted, `accent` to `accent_blue_hover` when muted

Shared buttons are `Component` structs that own their hover state, so call sites keep the same `ghost_button("Save", || ...)` shape. Do not add hover with `use_state` inside `apps/` renders. If a new hoverable shape is needed twice, add it to `libs/ui` once.

Other controls and when to reach for them:

- `segmented_control()` for inline pickers (Light/Dark/Auto in a card header). Hug-content. `segmented_control_full()` for full-width rows (DND schedule presets in the calendar panel) with equal `flex(1)` segments. Same split for `segmented_control_dynamic()` and `segmented_control_dynamic_full()`. Never put the hug variant in a full-width row and never wrap the full variant in an extra flex rect.
- `slider_row()` for brightness and volume. Already full width. `slider_row_with_mute()` adds the mute pill with hover.
- `status_chip(text, is_on, on_press)` for Connected, Paired, INSTALLED, DEFAULT, kind badges; never hand roll `rect().padding().corner_radius(999)` badges. Chips with `on_press` get hover. Display-only chips stay flat.
- `primary_button()`, `secondary_button()`, `danger_button()`, `ghost_button()`, `focus_pill()` for actions; never `Button::new()` in overlays, never accent `rect().on_press` (locker Unlock did this), never bordered icon `rect().on_press` (files toolbar did this); don't wrap them in extra bordered rects. The `*_full` variants fill the parent. In a horizontal row of equals (media controls, Cut/Copy), give each button its own flex share and let the `*_full` button fill it.
- `ghost_icon_button()` and `ghost_icon_button_full()` for icon-only pill actions. Use for media Prev/Play-Pause/Next (`SKIP_BACK`, `PLAY`/`PAUSE`, `SKIP_FORWARD`), never text labels for transport controls.
- `icon_button(icon, on_press)` for toolbar icons; never hand roll `rect().padding(8).corner_radius(8)` icon boxes. Toolbar navigation uses SVG icons through `tool_button` (`CHEVRON_LEFT`, `CHEVRON_RIGHT`, `REFRESH_CW`), never text glyphs like `‹` `›` `↻`. The files toolbar shipped those glyphs and they are gone.
- Tab close uses `small_close_button()`, never `ghost_button("×")`.
- `ctx_button()` and `ctx_divider()` for context menus; never raw `MenuButton::new()`
- `input_level_pill()` and `multi_segment_bar()` for OSD and storage bars; never hand roll `rect().height(8).corner_radius(999)` meters
- `top_bar_custom()` for app top bars; never hand roll `rect().height(56)` chrome
- `list_item()` and `list_item_with_subtitle()` plus `status_chip` for launcher result rows
- `select_row()` and `dropdown_select()` for static-value pickers like First Day of Week
- `quick_action_tile()` for session and screenshot tiles; add a flex-width variant rather than copying `session_btn` helpers

If a control needs a full-width variant (Cut, Copy, Duplicate in files), make the button itself fill. Don't wrap a fixed button in `rect().width(flex)`. The files properties drawer shipped uneven buttons for months because of that wrapper.

## Inputs

Freya `Input` draws its own border, so only one border per field.

Correct pill search pattern (see `sidebar_search()` in `libs/ui/src/components/sidebar.rs`): parent `rect` with corner radius `RADIUS_PILL`, background `panel`, border `border`, plus inner `rect width flex` with `content Flex`, plus `Input` with transparent background, transparent border fill, transparent focus background, transparent focus border fill.

What went wrong before:

- `shortcuts.rs` used raw `Input width flex` with no wrapper. Single border but wrong style.
- `language.rs` had the right wrapper but inner `Input` had no `width fill`. Parent expanded, input didn't, and text clipped to "Search installed lo".
- `files/main.rs` used `Input width px 140`. Too narrow and cramped. Use `fill` or `flex`.
- `files/main.rs` path field with `.flat()` inside a bordered parent was correct. Copy that.

Rule: styled parent means transparent Input plus `width Size::fill()`. Unstyled row means pill search instead of a bare Input. Never fixed `px` widths for search. Any `rect` with `flex` children needs `.content(Content::Flex)` per the `freya` skill or layout silently breaks.

## Lists and Grids

Rows: `setting_row(name, meta, has_top_border, control)` with `content Flex`. It handles title plus dim meta plus right control.

Layout helpers from `libs/ui/src/components/layout.rs`: `responsive_view(breakpoint, |compact|)`, `responsive_stack(compact, [...])`, `horizontal_stack`, `vertical_stack`, `grid2`. Overview uses 1000.0, detail pages use 720.0 to 1000.0, settings root uses 780.0.

Files grid drift came from fixed `120x140` cells, `wrap_spacing 6`, icons at 24/32/48. Tiny icons in a loose grid. Keep cells tight, icons larger, spacing from `GAP`. Avoid fixed `220 / 56 / 280` chrome where flex works. Sidebar at `flex(0.6).max_width(260)` survives 640px windows; `px(220)` doesn't.

Context menus: build rows with `ctx_button`, separate groups with `ctx_divider`, open with `ContextMenu::open_from_down`.

## Dialogs, Empty States, Status

Modal cards (`overlay/modal.rs`, `osd.rs`, `session.rs`, centered 380 to 520px): use `tile()` today; extract `dialog_card()` when you touch them twice. Never duplicate `width(400).padding(20).corner_radius(16)` per dialog.

Empty states (No clipboard items, No notifications, launcher hints): `tile_sub()` text today; extract `empty_state(icon, title, subtitle)` on repeat. Launcher already has one, use it.

Section labels: `field_label()` or `nav_group_label()`, never manual 11 to 13px BOLD `text_muted` ALL CAPS.

Status dot rows (wifi/BT dot plus Connected): `setting_row()` or `dot_status_row()`, don't rebuild the dot.

Spec triplets and hero rows (Used/Available/Total, avatar plus name): `stat_block()` or `hero_identity()` rather than copying label/value rects.

Notice patterns that now exist in files and store: `notice_pill(msg, kind)` for inline status, `NoticeBanner` with Details toggle plus Copy log and Save log for store errors. Don't send users to the terminal as the primary path. "Full log in terminal as STORE_ERROR_JSON" shipped once and rightly got flagged.

## Motion

Every animation must answer one question: where did this come from, did my action register, or what do I read first. If it answers none of those, delete it.

Primitives live in `libs/ui/src/motion/`. Never hand roll `use_animation` in `apps/`.

- `FadeSlideIn` for panel and card enter (opacity plus 8px rise, optional scale from)
- `FadeIn` for opacity only
- `PanelShell::new(dismiss)` for enter plus deferred exit; it fires `on_dismiss` after the fade so `close_window` never pops
- `AnimatedPage::new(route_key)` plus `PageDirection` for settings subpage enter
- `PressScale` for press feedback wrapper
- `stagger_delay(index, step, max)` for short list enter
- Tokens: `DUR_FAST 120`, `DUR_STANDARD 170`, `DUR_EMPHASIS 240`; curves Expo-Out for panels, Cubic-Out for pages

Good: panel opens with `PanelShell`, 170ms Expo-Out, opacity plus rise plus 0.97 scale. Settings subpage enters with `AnimatedPage`, Forward slides from +24px, Back from -24px, 240ms Cubic-Out.

Bad: 500ms elastic panel bounce, per-row stagger on a 50-row scroll list, animating `width` or `height` so the whole layout reflows mid-transition.

Rules:

- One motion per transition. Enter is opacity plus one axis. Never opacity plus slide plus scale plus color at once.
- Prefer `opacity`, `offset_x`/`offset_y`, `scale`. They composite. Width and height reflow layout and jank on large trees.
- Keep hover instant. Hover is a state query the pointer answers every frame; a fade on hover always lags the hand. Animate selection and activation, not hover.
- Press feedback goes on the way down fast (`PressScale`, 120ms) or not at all. Never delay the press response to wait for an animation.
- Never block input on animation. No modal waits, no sleep before `close_window`. `PanelShell` defers close by about 170ms after the user already acted; it never holds the action hostage.
- Stagger is for short entering lists only (5 to 8 items, 20 to 30ms step, capped near 120ms total). Never stagger on scroll or filter keystrokes.
- No loops, no Bounce/Elastic/Back for standard UI. Reserve them for playful one-shots an adult could defend in review.
- Exit mirrors enter, same curve reversed, same duration or shorter. If there is no exit path (`PanelShell` dismiss, `AnimatedPage` settle), there is no enter animation either.
- Reduced motion: every motion component takes `.reduced(true)`. When you touch a settings toggle for motion sensitivity, thread it through as `reduced` instead of adding a second code path.
- Animation hooks live in `libs/ui` components (`SwitchThumb`, `NavBody` own theirs), never in app loops, conditionals, or event handlers. If `apps/` needs a new animated shape, add the component to `libs/ui` once.

A mistake worth remembering: fading a whole settings page on every keystroke in its search field because enter keyed off filter text instead of `route_key`. Key enter off identity, not input.

## Responsive and Scale

Minimum window is 640x480. Text must hold at 100, 125, 150, and 200 percent scale.

- Sidebar collapses or narrows under the breakpoint; it never pushes content off screen.
- Search and path fields use `flex` or `fill`; never fixed pixel widths.
- Grid cells use `flex` with `max_width`, never fixed 120x140.
- Drawers use `flex` with `max_width(340)`, never fixed 280.
- Panels scroll internally (`ScrollView`) when content exceeds available height; they never grow the window.

If you find yourself writing `Size::px(220)` for chrome, stop and use a flex value with a max.

## Accessibility Baseline

Most clickable `rect()` controls shipped without semantic labels or focus states. That gap is now explicit.

- Focusable elements need a stable `AccessibilityId` from `use_a11y()`, attached with `.a11y_id()` and `.a11y_focusable(true)`, tracked with `use_focus(id)`. Show a ring for `Focus::Keyboard`; pointer focus needs no ring.
- `on_press` already covers click, tap, and Enter/Space, so keyboard activation comes free. Use `on_mouse_*` only when you need pointer-specific behavior like drag handles.
- Every interactive control gets a visible focus state and a semantic label or tooltip. No critical action depends on hover.
- Minimum hit targets: toolbar icons 28px or larger, list rows 44px or taller where practical, close buttons never smaller than the shared helper.
- Respect `reduce_motion`, `increase_contrast`, `reduce_transparency` from the store. Thread them through as flags; don't fork components into two variants.

## Copy

Headers and buttons carry the product voice, so keep them plain and specific. "Block unauthorized network access" promised enforcement the firewall toggle didn't have and had to be rewritten as "Preview only." Prefer verbs that match what the code does: Save, Apply, Retry, Reveal in folder, Copy log, Run command.

## Checklist Before Ship

1. Did you use `tile`, `tile_head`, `setting_row`, `page_head` instead of manual rects.
2. Any raw `Switch`, `Button`, `MenuButton`, or raw `Input` left. Replace with `pill_switch`, shared buttons, `ctx_button`, or pill search. Shared buttons carry hover internally, nothing extra to wire at the call site.
3. Any hand-rolled close `✕`, badge, bar, top bar, or result row. Replace with `small_close_button`, `status_chip`, `input_level_pill` or `multi_segment_bar`, `top_bar_custom`, `list_item`. Transport and toolbar actions use SVG icon buttons (`ghost_icon_button_full`, `tool_button`), never text labels or text glyphs.
4. One border per input. Transparent Input in styled parents, full width.
5. Tokens from the short set, `GAP` spacing, `RADIUS_LG/MD/SM/PILL`.
6. No fixed search widths, no uneven button wraps, responsive helpers where Overview uses them.
7. If you copied a rect twice, did you move it to `libs/ui` instead.
8. Every animation uses a `motion/` primitive with a token duration, answers where/confirmed/first-read, and has a matching exit or settle path.
9. No hover fades, no layout-size animation, no stagger on scroll or keystrokes, no input blocked on motion.
