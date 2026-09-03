## Layout conventions

- In Freya layouts, any `rect()` with children using `Size::flex(...)` must also set `.content(Content::Flex)` on that immediate parent container.