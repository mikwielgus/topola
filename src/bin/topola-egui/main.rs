#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod action;
mod activity;
mod app;
mod bottom;
mod error_dialog;
mod file_receiver;
mod file_sender;
mod layers;
mod overlay;
mod painter;
mod top;
mod translator;
mod viewport;

use app::App;
use fluent_templates::static_loader;
use sys_locale::get_locale;
use unic_langid::{langid, LanguageIdentifier};

// Build to native.
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let langid = if let Some(langname) = get_locale() {
        langname.parse().unwrap_or(langid!("en-US"))
    } else {
        langid!("en-US")
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };
    eframe::run_native(
        "topola-egui",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc, langid)))),
    )
}

// Build to Web.
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log`:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let start_result = eframe::WebRunner::new()
            .start(
                "topola-egui",
                web_options,
                Box::new(|cc| Ok(Box::new(App::new(cc, langid!("en-US"))))),
            )
            .await;

        // Remove the loading text and spinner:
        let loading_text = eframe::web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("loading_text"));
        if let Some(loading_text) = loading_text {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
