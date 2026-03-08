/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    view_rect: egui::Rect,
}

impl Default for App {
    fn default() -> Self {
        Self {
            view_rect: egui::Rect::from_min_max(
                egui::pos2(-100.0, 100.0),
                egui::pos2(100.0, 100.0),
            ),
        }
    }
}

impl App {
    /// Called once on start.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Restore the persistent part of the app's state from its previous run
        // if there is one.
        if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        }
    }
}

impl eframe::App for App {
    /// Called to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI has to be repainted.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    // Quit is not present on Web; the app can't close a webpage
                    // by itself.
                    if !cfg!(target_arch = "wasm32") {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                });

                ui.separator();

                egui::widgets::global_theme_preference_switch(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Scene::new()
                .zoom_range(0.001..=1000.0)
                .show(ui, &mut self.view_rect, |ui| {
                    ui.painter()
                        .circle_filled(egui::pos2(0.0, 0.0), 20.0, egui::Color32::RED);
                });
        });
    }
}
