//! A circular participant bubble: video (or initials), a speaking glow, and
//! small status badges.

use gpui::{
    AnyElement, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, hsla,
    linear_color_stop, linear_gradient, prelude::*, px, svg,
};

use crate::call::Participant;
use crate::ui::theme;

/// `is_active_speaker` adds a glow to the loudest speaker; anyone else who
/// speaks gets only the green ring.
pub fn bubble(p: &Participant, diameter: f32, is_active_speaker: bool) -> AnyElement {
    let speaking = p.is_speaking();
    let initials: SharedString = p.initials().into();
    let first_name: SharedString = p
        .name
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string()
        .into();
    let label: SharedString = if p.is_local { "You".into() } else { first_name };
    let badge = (diameter * 0.26).clamp(20., 30.);
    let group: SharedString = format!("bubble-{}", p.id.0).into();

    let face = div()
        .size_full()
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme::text())
        .map(|face| {
            if p.camera_on {
                // Stand-in for a video frame until real media arrives (see docs/PLAN.md).
                face.bg(linear_gradient(
                    135.,
                    linear_color_stop(hsla(p.hue, 0.55, 0.58, 1.0), 0.),
                    linear_color_stop(hsla((p.hue + 0.12) % 1.0, 0.60, 0.32, 1.0), 1.),
                ))
                .text_size(px(diameter * 0.30))
                .child(initials.clone())
            } else {
                face.bg(hsla(p.hue, 0.25, 0.18, 1.0))
                    .border_2()
                    .border_color(hsla(p.hue, 0.55, 0.55, 1.0))
                    .text_color(hsla(p.hue, 0.70, 0.75, 1.0))
                    .text_size(px(diameter * 0.32))
                    .child(initials.clone())
            }
        });

    div()
        .id(SharedString::from(format!("participant-{}", p.id.0)))
        .group(group.clone())
        .relative()
        .flex_none()
        .size(px(diameter))
        .rounded_full()
        .map(|b| {
            if is_active_speaker {
                b.border_3()
                    .border_color(theme::speaking())
                    .shadow(theme::speaking_glow(p.audio_level))
            } else if speaking {
                b.border_3()
                    .border_color(theme::speaking())
                    .shadow(theme::soft_shadow())
            } else {
                b.border_3()
                    .border_color(hsla(0., 0., 1., 0.10))
                    .shadow(theme::soft_shadow())
            }
        })
        .child(face)
        // Name pill, shown on hover like in Around.
        .child(
            div()
                .absolute()
                .bottom(px(-4.))
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .invisible()
                .group_hover(group, |s| s.visible())
                .child(
                    div()
                        .px_2()
                        .rounded_full()
                        .bg(theme::surface())
                        .text_color(theme::text())
                        .text_xs()
                        .child(label),
                ),
        )
        .when(!p.mic_on, |b| {
            b.child(
                status_badge("icons/mic-off.svg", theme::danger(), badge)
                    .right(px(0.))
                    .bottom(px(0.)),
            )
        })
        .when(p.screen_sharing, |b| {
            b.child(
                status_badge("icons/screen.svg", theme::accent(), badge)
                    .right(px(0.))
                    .top(px(0.)),
            )
        })
        .into_any_element()
}

fn status_badge(icon: &'static str, color: gpui::Hsla, size: f32) -> gpui::Div {
    div()
        .absolute()
        .size(px(size))
        .rounded_full()
        .bg(color)
        .border_2()
        .border_color(theme::surface())
        .flex()
        .items_center()
        .justify_center()
        .child(
            svg()
                .path(icon)
                .size(px(size * 0.55))
                .text_color(theme::text()),
        )
}
