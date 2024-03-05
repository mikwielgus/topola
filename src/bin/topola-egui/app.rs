use futures::executor;
use std::{
    future::Future,
    sync::mpsc::{channel, Receiver, Sender},
};

use topola::{
    dsn::design::DsnDesign,
    geometry::shape::{BendShape, DotShape, SegShape, Shape},
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
    file_handle_channel: (Sender<rfd::FileHandle>, Receiver<rfd::FileHandle>),

    #[serde(skip)]
    design: Option<DsnDesign>,

    #[serde(skip)]
    zoom_factor: f32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            file_handle_channel: channel(),
            design: None,
            zoom_factor: 1.0,
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
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if let Ok(file_handle) = self.file_handle_channel.1.try_recv() {
                        dbg!(file_handle);
                        // TODO: actually load the file from the handle.
                    }

                    if ui.button("Open").clicked() {
                        // `Context` is cheap to clone as it's wrapped in an `Arc`.
                        let ctx = ui.ctx().clone();
                        // NOTE: On Linux, this requires Zenity to be installed on your system.
                        let sender = self.file_handle_channel.0.clone();
                        let task = rfd::AsyncFileDialog::new().pick_file();

                        execute(async move {
                            let maybe_file_handle = task.await;

                            if let Some(file_handle) = maybe_file_handle {
                                let _ = sender.send(file_handle);
                                ctx.request_repaint();
                            }
                            /*if let Some(file) = file {
                                let text = file.read().await;
                                let _ = sender.send(String::from_utf8_lossy(&text).to_string());
                                ctx.request_repaint();
                            }*/
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

                self.zoom_factor = self.zoom_factor * ctx.input(|i| i.zoom_delta());

                let transform = egui::emath::RectTransform::from_to(
                    egui::Rect::from_x_y_ranges(0.0..=1000.0, 0.0..=500.0) / self.zoom_factor,
                    viewport_rect,
                );
                let mut painter = Painter::new(ui, transform);

                let dot_shape = Shape::Dot(DotShape {
                    c: Circle {
                        pos: [50.0, 100.0].into(),
                        r: 10.0 * self.zoom_factor as f64,
                    },
                });

                let seg_shape = Shape::Seg(SegShape {
                    from: [200.0, 25.0].into(),
                    to: [300.0, 300.0].into(),
                    width: 5.0 * self.zoom_factor as f64,
                });

                let bend_shape = Shape::Bend(BendShape {
                    from: [100.0, 100.0].into(),
                    to: [160.0, 160.0].into(),
                    c: Circle {
                        pos: [130.0, 130.0].into(),
                        r: 30.0,
                    },
                    width: 12.0 * self.zoom_factor as f64,
                });

                painter.paint_shape(&dot_shape, egui::Color32::from_rgb(255, 0, 0));
                painter.paint_shape(&seg_shape, egui::Color32::from_rgb(128, 128, 128));
                painter.paint_shape(&bend_shape, egui::Color32::from_rgb(255, 255, 0));
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
