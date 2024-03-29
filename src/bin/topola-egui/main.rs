#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
mod painter;
use app::App;

// Build to native.
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };
    eframe::run_native(
        "topola-egui",
        native_options,
        Box::new(|cc| Box::new(App::new(cc))),
    )
}

// Build to Web.
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log`:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "topola-egui",
                web_options,
                Box::new(|cc| Box::new(App::new(cc))),
            )
            .await
            .expect("failed to start eframe");
    });
}
