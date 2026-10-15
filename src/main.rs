mod assets;
mod call;
mod media;
mod ui;

use gpui::{Application, KeyBinding};

use ui::call_window::{LeaveCall, ToggleCamera, ToggleMic};

fn main() {
    env_logger::init();
    Application::new().with_assets(assets::Assets).run(|cx| {
        cx.bind_keys([
            KeyBinding::new("secondary-d", ToggleMic, Some("CallWindow")),
            KeyBinding::new("secondary-e", ToggleCamera, Some("CallWindow")),
            KeyBinding::new("secondary-shift-h", LeaveCall, Some("CallWindow")),
        ]);
        // Quit when the last window closes, on every platform.
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        // `--demo` skips the lobby and joins the simulated demo room.
        if std::env::args().any(|arg| arg == "--demo") {
            let call = call::CallState::new("demo", "You");
            ui::call_window::open(call, Box::new(media::SimulatedRoom::new(7)), cx)
                .expect("failed to open the call window");
        } else {
            ui::lobby::open(cx).expect("failed to open the lobby window");
        }
        cx.activate(true);
    });
}
