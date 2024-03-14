use futures::executor;
use std::{
    future::Future,
    sync::mpsc::{channel, Receiver, Sender},
};

use topola::{
    dsn::{design::DsnDesign, rules::DsnRules},
    geometry::shape::{BendShape, DotShape, SegShape, Shape},
    layout::{graph::MakePrimitive, primitive::MakeShape, Layout},
    math::Circle,
};

use crate::painter::Painter;

/// Deserialize/Serialize is needed to persist app state between restarts.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct App {
    // Example stuff:
    label: String,

    #[serde(skip)] // Don't serialize this field.
    text_channel: (Sender<String>, Receiver<String>),

    #[serde(skip)]
    layout: Option<Layout<DsnRules>>,

    #[serde(skip)]
    from_rect: egui::emath::Rect,
}

impl Default for App {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            text_channel: channel(),
            layout: None,
            from_rect: egui::Rect::from_x_y_ranges(0.0..=1000000.0, 0.0..=500000.0),
        }
    }
}

impl App {
    /// Called once on start.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load previous app state if one exists.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for App {
    /// Called to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI has to be repainted.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if cfg!(target_arch = "wasm32") {
            if let Ok(file_contents) = self.text_channel.1.try_recv() {
                let design = DsnDesign::load_from_string(file_contents).unwrap();
                self.layout = Some(design.make_layout());
            }
        } else {
            if let Ok(path) = self.text_channel.1.try_recv() {
                let design = DsnDesign::load_from_file(&path).unwrap();
                self.layout = Some(design.make_layout());
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        // `Context` is cheap to clone as it's wrapped in an `Arc`.
                        let ctx = ui.ctx().clone();
                        // NOTE: On Linux, this requires Zenity to be installed on your system.
                        let sender = self.text_channel.0.clone();
                        let task = rfd::AsyncFileDialog::new().pick_file();

                        execute(async move {
                            let maybe_file_handle = task.await;

                            if let Some(file_handle) = maybe_file_handle {
                                let _ = sender.send(channel_text(file_handle).await);
                                ctx.request_repaint();
                            }
                        });
                    }

                    // "Quit" button wouldn't work on a Web page.
                    if !cfg!(target_arch = "wasm32") {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                });
                ui.add_space(16.0);

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                ui.ctx().request_repaint();

                let desired_size = ui.available_width() * egui::vec2(1.0, 0.5);
                let (_id, viewport_rect) = ui.allocate_space(desired_size);

                let old_transform =
                    egui::emath::RectTransform::from_to(self.from_rect, viewport_rect);
                let latest_pos = old_transform
                    .inverse()
                    .transform_pos(ctx.input(|i| i.pointer.latest_pos().unwrap_or_default()));

                let old_scale = old_transform.scale().x;
                self.from_rect = self.from_rect / ctx.input(|i| i.zoom_delta());

                let new_scale = egui::emath::RectTransform::from_to(self.from_rect, viewport_rect)
                    .scale()
                    .x;

                self.from_rect = self.from_rect.translate(
                    ctx.input(|i| latest_pos.to_vec2() * (new_scale - old_scale) / new_scale),
                );

                self.from_rect = self
                    .from_rect
                    .translate(ctx.input(|i| -i.raw_scroll_delta / new_scale));

                let transform = egui::emath::RectTransform::from_to(self.from_rect, viewport_rect);
                let mut painter = Painter::new(ui, transform);

                if let Some(layout) = &self.layout {
                    for node in layout.layer_nodes(1) {
                        let shape = node.primitive(layout).shape();
                        painter.paint_shape(&shape, egui::Color32::from_rgb(52, 52, 200));
                    }

                    for node in layout.layer_nodes(0) {
                        let shape = node.primitive(layout).shape();
                        painter.paint_shape(&shape, egui::Color32::from_rgb(200, 52, 52));
                    }
                }
            })
        });

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    std::thread::spawn(move || futures::executor::block_on(f));
}

#[cfg(target_arch = "wasm32")]
fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

#[cfg(not(target_arch = "wasm32"))]
async fn channel_text(file_handle: rfd::FileHandle) -> String {
    file_handle.path().to_str().unwrap().to_string()
}

#[cfg(target_arch = "wasm32")]
async fn channel_text(file_handle: rfd::FileHandle) -> String {
    std::str::from_utf8(&file_handle.read().await)
        .unwrap()
        .to_string()
}
