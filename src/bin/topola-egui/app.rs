use serde::{Deserialize, Serialize};
use std::{
    future::Future,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
};
use unic_langid::{langid, LanguageIdentifier};

use topola::{
    autorouter::{invoker::Invoker, Autorouter},
    specctra::{design::SpecctraDesign, mesadata::SpecctraMesadata},
    stepper::Step,
};

use crate::{
    activity::{ActivityStatus, ActivityStepperWithStatus},
    error_dialog::ErrorDialog,
    file_receiver::FileReceiver,
    layers::Layers,
    menu_bar::MenuBar,
    overlay::Overlay,
    status_bar::StatusBar,
    translator::Translator,
    viewport::Viewport,
};

/// Deserialize/Serialize is needed to persist app state between restarts.
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct App {
    translator: Translator,

    #[serde(skip)]
    maybe_overlay: Option<Overlay>,

    #[serde(skip)]
    arc_mutex_maybe_invoker: Arc<Mutex<Option<Invoker<SpecctraMesadata>>>>,

    #[serde(skip)]
    maybe_activity: Option<ActivityStepperWithStatus>,

    #[serde(skip)]
    content_channel: (Sender<String>, Receiver<String>),

    #[serde(skip)]
    history_channel: (Sender<String>, Receiver<String>),

    #[serde(skip)]
    viewport: Viewport,

    #[serde(skip)]
    menu_bar: MenuBar,

    #[serde(skip)]
    status_bar: StatusBar,

    #[serde(skip)]
    error_dialog: ErrorDialog,

    #[serde(skip)]
    maybe_layers: Option<Layers>,

    #[serde(skip)]
    maybe_design: Option<SpecctraDesign>,

    #[serde(skip)]
    update_counter: f32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            translator: Translator::new(langid!("en-US")),
            maybe_overlay: None,
            arc_mutex_maybe_invoker: Arc::new(Mutex::new(None)),
            maybe_activity: None,
            content_channel: channel(),
            history_channel: channel(),
            viewport: Viewport::new(),
            menu_bar: MenuBar::new(),
            status_bar: StatusBar::new(),
            error_dialog: ErrorDialog::new(),
            maybe_layers: None,
            maybe_design: None,
            update_counter: 0.0,
        }
    }
}

impl App {
    /// Called once on start.
    pub fn new(cc: &eframe::CreationContext<'_>, langid: LanguageIdentifier) -> Self {
        // Load previous app state if one exists.
        if let Some(storage) = cc.storage {
            let this = Self {
                translator: Translator::new(langid),
                ..eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
            };
            return this;
        }

        Self {
            translator: Translator::new(langid),
            ..Default::default()
        }
    }

    fn advance_state_by_dt(&mut self, dt: f32) {
        self.update_counter += dt;

        while self.update_counter >= self.menu_bar.frame_timestep {
            self.update_counter -= self.menu_bar.frame_timestep;

            if !self.update_state() {
                return;
            }
        }
    }

    fn update_state(&mut self) -> bool {
        let mut content_file_receiver = FileReceiver::new(&self.content_channel.1);

        if let Some(input) = content_file_receiver.try_recv() {
            match self.load_specctra_dsn(input) {
                Ok(()) => {}
                Err(err) => {
                    self.error_dialog.push_error("specctra-dsn-loader", err);
                }
            }
        }

        if let Some(invoker) = self.arc_mutex_maybe_invoker.lock().unwrap().as_mut() {
            let mut history_file_receiver = FileReceiver::new(&self.history_channel.1);

            if let Some(input) = history_file_receiver.try_recv() {
                let tr = &self.translator;
                match input {
                    Ok(bufread) => match serde_json::from_reader(bufread) {
                        Ok(res) => invoker.replay(res),
                        Err(err) => {
                            self.error_dialog.push_error(
                                "history-loader",
                                format!("{}; {}", tr.text("error-file-history-parse"), err),
                            );
                        }
                    },
                    Err(err) => {
                        self.error_dialog.push_error(
                            "history-loader",
                            format!("{}; {}", tr.text("error-file-load"), err),
                        );
                    }
                }
            }

            if let Some(ref mut activity) = self.maybe_activity {
                return match activity.step(invoker) {
                    Ok(ActivityStatus::Running) => true,
                    Ok(ActivityStatus::Finished(..)) => false,
                    Err(err) => {
                        self.error_dialog.push_error("invoker", format!("{}", err));
                        false
                    }
                };
            }
        }

        false
    }

    fn load_specctra_dsn(
        &mut self,
        input: std::io::Result<std::io::BufReader<std::fs::File>>,
    ) -> Result<(), String> {
        let tr = &self.translator;
        let bufread = input.map_err(|err| format!("{}; {}", tr.text("error-file-load"), err))?;
        let design = SpecctraDesign::load(bufread)
            .map_err(|err| format!("{}; {}", tr.text("error-file-specctra-dsn-parse"), err))?;
        let board = design.make_board();
        let overlay = Overlay::new(&board)
            .map_err(|err| format!("{}; {}", tr.text("error-overlay-init"), err))?;
        let layers = Layers::new(&board);
        let autorouter = Autorouter::new(board)
            .map_err(|err| format!("{}; {}", tr.text("error-autorouter-init"), err))?;
        self.maybe_overlay = Some(overlay);
        self.maybe_layers = Some(layers);
        self.maybe_design = Some(design);
        self.arc_mutex_maybe_invoker = Arc::new(Mutex::new(Some(Invoker::new(autorouter))));
        self.viewport.scheduled_zoom_to_fit = true;
        Ok(())
    }
}

impl eframe::App for App {
    /// Called to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI has to be repainted.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.menu_bar.update(
            ctx,
            &self.translator,
            self.content_channel.0.clone(),
            self.history_channel.0.clone(),
            self.arc_mutex_maybe_invoker.clone(),
            &mut self.maybe_activity,
            &mut self.viewport,
            &mut self.maybe_overlay,
            &self.maybe_design,
        );

        self.advance_state_by_dt(ctx.input(|i| i.stable_dt));

        self.status_bar
            .update(ctx, &self.translator, &self.viewport, &self.maybe_activity);

        if self.menu_bar.show_layer_manager {
            if let Some(ref mut layers) = self.maybe_layers {
                if let Some(invoker) = self.arc_mutex_maybe_invoker.lock().unwrap().as_ref() {
                    layers.update(ctx, invoker.autorouter().board());
                }
            }
        }

        self.error_dialog.update(ctx, &self.translator);

        let _viewport_rect = self.viewport.update(
            ctx,
            &self.menu_bar,
            &mut self.arc_mutex_maybe_invoker.lock().unwrap(),
            &mut self.maybe_activity,
            &mut self.maybe_overlay,
            &self.maybe_layers,
        );

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    std::thread::spawn(move || futures_lite::future::block_on(f));
}

#[cfg(target_arch = "wasm32")]
pub fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}
