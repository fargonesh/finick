## Layout conventions

- In Freya layouts, any `rect()` with children using `Size::flex(...)` must also set `.content(Content::Flex)` on that immediate parent container.
- In Freya, Hook functions must follow these rules:
  1. You cannot call them conditionally

  The following is not allowed and will result in this runtime error.

  #[derive(PartialEq)]
  struct CoolComp(u8);

  impl Component for CoolComp {
      fn render(&self) -> impl IntoElement {
          if self.0 == 2 {
              let state = use_state(|| 5);
          }

          rect().into()
      }
  }

  2. You cannot call them in for-loops

  The following is not allowed and will result in this runtime error.

  #[derive(PartialEq)]
  struct CoolComp(u8);

  impl Component for CoolComp {
      fn render(&self) -> impl IntoElement {
          for i in 0..self.0 {
              let state = use_state(|| 5);
          }

          rect().into()
      }
  }

  3. You cannot call hooks inside other hooks, event handlers, they should be called in the top of `render` methods from components.

  The following is not allowed and will result in this runtime error.

  #[derive(PartialEq)]
  struct CoolComp(u8);

  impl Component for CoolComp {
      fn render(&self) -> impl IntoElement {
          use_side_effect(|| {
              let state = use_state(|| 5);
          })

          rect().into()
      }
  }