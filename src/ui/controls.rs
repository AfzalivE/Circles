//! The compact control bar: round icon buttons in a pill.

use gpui::{
    App, ClickEvent, ElementId, Hsla, IntoElement, ParentElement, SharedString, Styled, Window,
    div, prelude::*, px, svg,
};

use crate::ui::theme;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonTone {
    Normal,
    /// The control is in an "off" state that the user should notice (muted, camera off).
    Off,
    Active,
    Danger,
}

pub fn icon_button(
    id: impl Into<ElementId>,
    icon: &'static str,
    tooltip: &'static str,
    tone: ButtonTone,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let (bg, fg, hover): (Hsla, Hsla, Hsla) = match tone {
        ButtonTone::Normal => (
            gpui::transparent_black(),
            theme::text(),
            theme::surface_hover(),
        ),
        ButtonTone::Off => (
            theme::danger().opacity(0.18),
            theme::danger(),
            theme::danger().opacity(0.30),
        ),
        ButtonTone::Active => (
            theme::accent(),
            theme::text(),
            theme::accent().opacity(0.85),
        ),
        ButtonTone::Danger => (
            theme::danger(),
            theme::text(),
            theme::danger().opacity(0.85),
        ),
    };
    let tooltip = SharedString::from(tooltip);

    div()
        .id(id)
        .size(px(32.))
        .flex_none()
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(bg)
        .hover(move |s| s.bg(hover))
        .cursor_pointer()
        .tooltip(move |_, cx| cx.new(|_| Tooltip(tooltip.clone())).into())
        .on_click(on_click)
        .child(svg().path(icon).size(px(18.)).text_color(fg))
}

pub struct Tooltip(SharedString);

impl gpui::Render for Tooltip {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(theme::surface())
            .text_color(theme::text())
            .text_xs()
            .child(self.0.clone())
    }
}
