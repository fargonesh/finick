use crate::theme::use_app_theme;
use freya::prelude::*;

/// A standard page header component featuring a title and optional description.
#[derive(Clone, PartialEq)]
pub struct PageHeader {
    pub title: String,
    pub description: Option<String>,
}

impl PageHeader {
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), description: None }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

impl Component for PageHeader {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();
        let mut el = rect().margin((0., 0., 24., 0.)).child(
            label()
                .font_size(28.)
                .font_weight(FontWeight::BOLD)
                .color(t.text_primary)
                .margin((0., 0., 8., 0.))
                .text(self.title.clone()),
        );

        if let Some(desc) = &self.description {
            el = el.child(label().font_size(14.).color(t.text_secondary).text(desc.clone()));
        }

        el
    }
}

/// Helper function to create a standard `PageHeader`.
pub fn page_header(title: impl Into<String>, description: impl Into<String>) -> impl IntoElement {
    PageHeader::new(title).description(description)
}
