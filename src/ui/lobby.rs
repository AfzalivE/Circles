//! The start window: pick a room and join. The call itself happens in the
//! floating call window.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use gpui::{
    App, Bounds, Context, FontWeight, IntoElement, ParentElement, Render, SharedString, Styled,
    TitlebarOptions, Window, WindowBounds, WindowHandle, WindowOptions, div, prelude::*, px, size,
};

use crate::call::CallState;
use crate::media::SimulatedRoom;
use crate::ui::{call_window, theme};

pub struct Lobby {
    display_name: SharedString,
}

pub fn open(cx: &mut App) -> Result<WindowHandle<Lobby>> {
    let bounds = Bounds::centered(None, size(px(420.), px(340.)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions {
            title: Some("Circles".into()),
            ..Default::default()
        }),
        app_id: Some("circles".into()),
        ..Default::default()
    };
    cx.open_window(options, |_, cx| {
        cx.new(|_| Lobby {
            display_name: default_display_name().into(),
        })
    })
}

fn default_display_name() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .ok()
        .filter(|name| !name.is_empty())
        .map(|name| {
            let mut chars = name.chars();
            chars
                .next()
                .map(|c| c.to_uppercase().chain(chars).collect())
                .unwrap_or_default()
        })
        .unwrap_or_else(|| "Me".to_string())
}

/// A short room code such as `kq7-x2m`, like the codes Around puts in its links.
fn new_room_code() -> String {
    const ALPHABET: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";
    let mut n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(42);
    let mut code = String::new();
    for i in 0..6 {
        if i == 3 {
            code.push('-');
        }
        code.push(ALPHABET[(n % ALPHABET.len() as u64) as usize] as char);
        n /= ALPHABET.len() as u64;
        n = n.wrapping_mul(2862933555777941757).wrapping_add(3037000493);
    }
    code
}

impl Lobby {
    fn join(&mut self, room: String, window: &mut Window, cx: &mut Context<Self>) {
        let call = CallState::new(room, self.display_name.to_string());
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(1);
        match call_window::open(call, Box::new(SimulatedRoom::new(seed)), cx) {
            Ok(_) => window.remove_window(),
            Err(err) => eprintln!("could not open the call window: {err:#}"),
        }
    }
}

fn button(id: &'static str, label: &'static str, primary: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .w_full()
        .h(px(40.))
        .rounded_lg()
        .flex()
        .items_center()
        .justify_center()
        .font_weight(FontWeight::SEMIBOLD)
        .cursor_pointer()
        .map(|b| {
            if primary {
                b.bg(theme::accent())
                    .text_color(theme::text())
                    .hover(|s| s.bg(theme::accent().opacity(0.85)))
            } else {
                b.bg(theme::surface_hover())
                    .text_color(theme::text())
                    .hover(|s| s.bg(theme::surface_hover().opacity(0.8)))
            }
        })
        .child(label)
}

impl Render for Lobby {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .justify_center()
            .gap_3()
            .px_10()
            .bg(theme::surface().opacity(1.0))
            .text_color(theme::text())
            .child(
                div()
                    .text_size(px(28.))
                    .font_weight(FontWeight::BOLD)
                    .child("Circles"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme::text_muted())
                    .child("Video calls that float over your work."),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme::text_muted())
                    .child(format!("You join as {}.", self.display_name)),
            )
            .child(div().h_2())
            .child(button("start", "Start a call", true).on_click(
                cx.listener(|this, _, window, cx| this.join(new_room_code(), window, cx)),
            ))
            .child(
                button("demo", "Join the demo room", false).on_click(
                    cx.listener(|this, _, window, cx| this.join("demo".into(), window, cx)),
                ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_code_shape() {
        let code = new_room_code();
        assert_eq!(code.len(), 7);
        assert_eq!(code.as_bytes()[3], b'-');
    }
}
