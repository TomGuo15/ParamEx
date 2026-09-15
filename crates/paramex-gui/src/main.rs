//! ParamEx desktop entry point (egui/eframe).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use paramex_gui::app::ParamExApp;
use paramex_gui::platform;

fn main() -> eframe::Result {
    // 1) Logging first, so the single-instance log line and any startup
    //    failure are captured.
    let log_path = platform::init_logging();

    // Log every panic — including a caught IO-worker panic — to the app log.
    // Release builds unwind, so guarded workers can restore their queue
    // invariant; unhandled UI-thread panics still retain a crash trace.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("panic: {info}");
        default_hook(info);
    }));

    // 2) Single-instance: a second launch exits silently with no window.
    if platform::already_running() {
        tracing::info!("Another ParamEx instance is already running; exiting.");
        return Ok(());
    }

    // 3) Run the window. On any startup error, surface the native box naming
    //    the log path and propagate.
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            // The window may grow, but never shrink below the known-good
            // reference size, so the bento layout can only scale up from a
            // baseline that is guaranteed not to overlap.
            .with_min_inner_size([1280.0, 800.0])
            .with_title("ParamEx")
            .with_resizable(true)
            .with_icon(paramex_gui::theme::window_icon().unwrap_or_default()),
        ..Default::default()
    };

    let result = eframe::run_native(
        "ParamEx",
        native_options,
        Box::new(|cc| Ok(Box::new(ParamExApp::new(cc)))),
    );

    if let Err(err) = &result {
        tracing::error!("ParamEx failed to start: {err}");
        platform::show_startup_error(log_path.as_deref());
    }
    result
}
