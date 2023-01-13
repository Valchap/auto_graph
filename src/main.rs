#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// Lint settings
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::too_many_lines)]

mod app;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Auto Graph",
        native_options,
        Box::new(|_cc| Box::new(app::App::new())),
    );
}

// when building for web using trunk.
#[cfg(target_arch = "wasm32")]
fn main() {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();
    eframe::start_web(
        "the_canvas_id", // hardcode it
        web_options,
        Box::new(|_cc| Box::new(app::App::new())),
    )
    .expect("failed to start eframe");
}
