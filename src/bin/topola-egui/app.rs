/// Deserialize/Serialize is needed to persist app state between restarts.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct App {
    // Example stuff:
    label: String,

    #[serde(skip)] // Don't serialize this field.
    value: f32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
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
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(egui::Vec2::splat(300.0), egui::Sense::drag());
                let stroke = egui::Stroke::new(10.0, egui::Color32::from_white_alpha(255));
                ui.painter()
                    .line_segment([[10.0, 10.0].into(), [30.0, 30.0].into()], stroke);
            })
        });
    }
}
