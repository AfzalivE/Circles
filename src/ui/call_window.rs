//! The floating call window: a frameless, transparent, always-on-top window with
//! one bubble per participant and a compact control bar.

use std::time::Duration;

use anyhow::Result;
use gpui::{
    App, Bounds, Context, FocusHandle, Focusable, FontWeight, IntoElement, MouseButton,
    ParentElement, Render, Styled, Task, Window, WindowBackgroundAppearance, WindowBounds,
    WindowDecorations, WindowHandle, WindowKind, WindowOptions, actions, div, point, prelude::*,
    px, size,
};

use crate::call::{BUBBLE_GAP, CONTROLS_HEIGHT, CallState, Layout};
use crate::media::RoomBackend;
use crate::ui::bubble::bubble;
use crate::ui::controls::{ButtonTone, icon_button};
use crate::ui::{lobby, theme};

actions!(
    call,
    [
        ToggleMic,
        ToggleCamera,
        ToggleScreenShare,
        CycleBubbleSize,
        ToggleLayout,
        LeaveCall
    ]
);

const TICK: Duration = Duration::from_millis(50);
const SCREEN_MARGIN: f32 = 24.0;

/// GPUI cannot move a window from code yet, so the window grows to the right as
/// people join. We leave room for this many bubbles when we place it.
const PLACEMENT_PARTICIPANTS: usize = 4;

pub struct CallWindow {
    call: CallState,
    backend: Box<dyn RoomBackend>,
    focus: FocusHandle,
    window_size: (f32, f32),
    _ticker: Task<()>,
}

pub fn open(
    call: CallState,
    backend: Box<dyn RoomBackend>,
    cx: &mut App,
) -> Result<WindowHandle<CallWindow>> {
    let (w, h) = call.window_size();
    // Start in the top-right corner of the main screen, where Around puts its bubbles.
    let origin = match cx.primary_display() {
        Some(display) => {
            let screen = display.bounds();
            let room = call.window_size_for(PLACEMENT_PARTICIPANTS).0.max(w);
            point(
                screen.right() - px(room + SCREEN_MARGIN),
                screen.top() + px(SCREEN_MARGIN * 2.),
            )
        }
        None => point(px(SCREEN_MARGIN), px(SCREEN_MARGIN)),
    };
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            origin,
            size(px(w), px(h)),
        ))),
        titlebar: None,
        focus: true,
        show: true,
        kind: WindowKind::PopUp,
        is_movable: true,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Transparent,
        window_decorations: Some(WindowDecorations::Client),
        app_id: Some("around".into()),
        ..Default::default()
    };
    cx.open_window(options, |window, cx| {
        cx.new(|cx| CallWindow::new(call, backend, window, cx))
    })
}

impl CallWindow {
    fn new(
        call: CallState,
        backend: Box<dyn RoomBackend>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus);
        window.set_window_title(&format!("Around: {}", call.room));

        let ticker = cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                let alive = this.update_in(cx, |this, window, cx| this.tick(window, cx));
                if alive.is_err() {
                    break;
                }
            }
        });

        Self {
            window_size: call.window_size(),
            call,
            backend,
            focus,
            _ticker: ticker,
        }
    }

    fn tick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.call.elapsed += TICK;
        for event in self.backend.poll(TICK) {
            self.call.apply(event);
        }
        self.fit_window(window);
        cx.notify();
    }

    /// Resizes the window when the bubbles or layout need a different size.
    fn fit_window(&mut self, window: &mut Window) {
        let wanted = self.call.window_size();
        if wanted != self.window_size {
            self.window_size = wanted;
            window.resize(size(px(wanted.0), px(wanted.1)));
        }
    }

    fn toggle_mic(&mut self, _: &ToggleMic, _: &mut Window, cx: &mut Context<Self>) {
        let on = self.call.toggle_mic();
        self.backend.set_mic(on);
        cx.notify();
    }

    fn toggle_camera(&mut self, _: &ToggleCamera, _: &mut Window, cx: &mut Context<Self>) {
        let on = self.call.toggle_camera();
        self.backend.set_camera(on);
        cx.notify();
    }

    fn toggle_screen_share(
        &mut self,
        _: &ToggleScreenShare,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.call.toggle_screen_share();
        cx.notify();
    }

    fn cycle_bubble_size(
        &mut self,
        _: &CycleBubbleSize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.call.cycle_bubble_size();
        self.fit_window(window);
        cx.notify();
    }

    fn toggle_layout(&mut self, _: &ToggleLayout, window: &mut Window, cx: &mut Context<Self>) {
        self.call.toggle_layout();
        self.fit_window(window);
        cx.notify();
    }

    fn leave(&mut self, _: &LeaveCall, window: &mut Window, cx: &mut Context<Self>) {
        self.backend.leave();
        lobby::open(cx).ok();
        window.remove_window();
    }

    fn render_controls(&self) -> impl IntoElement {
        let local = self.call.local();
        let tone = |on: bool| {
            if on {
                ButtonTone::Normal
            } else {
                ButtonTone::Off
            }
        };

        div()
            .id("controls")
            .h(px(CONTROLS_HEIGHT))
            .flex_none()
            .flex()
            .items_center()
            .gap_1()
            .px(px(6.))
            .rounded_full()
            .bg(theme::surface())
            .shadow(theme::soft_shadow())
            // Clicks on the bar are for the buttons, not for dragging the window.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .w(px(48.))
                    .flex()
                    .justify_center()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme::text_muted())
                    .child(self.call.elapsed_label()),
            )
            .child(icon_button(
                "mic",
                if local.mic_on {
                    "icons/mic.svg"
                } else {
                    "icons/mic-off.svg"
                },
                "Mute or unmute (Ctrl/Cmd+D)",
                tone(local.mic_on),
                |_, window, cx| window.dispatch_action(Box::new(ToggleMic), cx),
            ))
            .child(icon_button(
                "camera",
                if local.camera_on {
                    "icons/camera.svg"
                } else {
                    "icons/camera-off.svg"
                },
                "Camera on or off (Ctrl/Cmd+E)",
                tone(local.camera_on),
                |_, window, cx| window.dispatch_action(Box::new(ToggleCamera), cx),
            ))
            .child(icon_button(
                "screen",
                "icons/screen.svg",
                "Share screen",
                if local.screen_sharing {
                    ButtonTone::Active
                } else {
                    ButtonTone::Normal
                },
                |_, window, cx| window.dispatch_action(Box::new(ToggleScreenShare), cx),
            ))
            .child(icon_button(
                "size",
                "icons/bubble-size.svg",
                "Bubble size",
                ButtonTone::Normal,
                |_, window, cx| window.dispatch_action(Box::new(CycleBubbleSize), cx),
            ))
            .child(icon_button(
                "layout",
                "icons/layout.svg",
                "Row or column",
                ButtonTone::Normal,
                |_, window, cx| window.dispatch_action(Box::new(ToggleLayout), cx),
            ))
            .child(icon_button(
                "leave",
                "icons/leave.svg",
                "Leave call",
                ButtonTone::Danger,
                |_, window, cx| window.dispatch_action(Box::new(LeaveCall), cx),
            ))
    }
}

impl Focusable for CallWindow {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for CallWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let diameter = self.call.bubble_size.diameter();
        let active = self.call.active_speaker();
        let bubbles = self
            .call
            .participants
            .iter()
            .map(|p| bubble(p, diameter, Some(p.id) == active));

        div()
            .id("call-window")
            .key_context("CallWindow")
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::toggle_mic))
            .on_action(cx.listener(Self::toggle_camera))
            .on_action(cx.listener(Self::toggle_screen_share))
            .on_action(cx.listener(Self::cycle_bubble_size))
            .on_action(cx.listener(Self::toggle_layout))
            .on_action(cx.listener(Self::leave))
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(BUBBLE_GAP))
            .p(px(BUBBLE_GAP))
            // The window has no title bar: drag anywhere on the bubbles to move it.
            .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
            .child(
                div()
                    .flex()
                    .gap(px(BUBBLE_GAP))
                    .map(|d| match self.call.layout {
                        Layout::Row => d.flex_row(),
                        Layout::Column => d.flex_col(),
                    })
                    .children(bubbles),
            )
            .child(self.render_controls())
    }
}
