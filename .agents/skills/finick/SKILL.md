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
`status_chip()` for Connected, Paired.
`primary_button()`, `secondary_button()`, `danger_button()`, `ghost_button()`, `focus_pill()` for actions. Do not wrap them in extra bordered rects.

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

## Checklist before ship

1. Did I use `tile`, `tile_head`, `setting_row`, `page_head` instead of manual rects.
2. Any raw `Switch` or raw `Input` left. Replace with `pill_switch` or pill search.
3. One border per input. Transparent Input in styled parents, full width.
4. Tokens from the short set, `GAP` spacing, `RADIUS_LG/MD/SM/PILL`.
5. No fixed search widths, no uneven button wraps, responsive helpers where Overview uses them.
6. Copy passes `anti-ai-slop-writing`, components pass `freya`.
