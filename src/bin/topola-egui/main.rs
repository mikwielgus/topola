#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod action;
mod activity;
mod app;
mod config;
mod error_dialog;
mod file_handler;
mod layers;
mod menu_bar;
mod overlay;
mod painter;
mod status_bar;
mod translator;
mod viewport;

use app::App;
use sys_locale::get_locale;
use unic_langid::{langid, LanguageIdentifier};

fn get_langid() -> LanguageIdentifier {
    get_locale()
        .and_then(|langname| langname.parse().ok())
        .unwrap_or(langid!("en-US"))
}

// Build to native.
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let langid = get_langid();

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
    use eframe::wasm_bindgen::JsCast;

    // Redirect `log` message to `console.log`:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();
    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = eframe::web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("topola-egui")
            .expect("Failed to find the canvas id")
            .dyn_into::<eframe::web_sys::HtmlCanvasElement>()
            .expect("topola-egui was not a HtmlCanvasElement");

        let langid = get_langid();
        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(App::new(cc, langid)))),
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
