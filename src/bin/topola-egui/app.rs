use serde::{Deserialize, Serialize};
use std::{
    future::Future,
    io,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
};
use unic_langid::{langid, LanguageIdentifier};

use topola::{
    autorouter::{history::History, invoker::Invoker, Autorouter},
    specctra::{
        design::{LoadingError as SpecctraLoadingError, SpecctraDesign},
        mesadata::SpecctraMesadata,
    },
    stepper::Step,
};

use crate::{
    activity::{ActivityContext, ActivityStatus, ActivityStepperWithStatus},
    config::Config,
    error_dialog::ErrorDialog,
    interaction::InteractionContext,
    layers::Layers,
    menu_bar::MenuBar,
    overlay::Overlay,
    status_bar::StatusBar,
    translator::Translator,
    viewport::Viewport,
};

pub struct App {
    config: Config,
    translator: Translator,

    maybe_overlay: Option<Overlay>,

    arc_mutex_maybe_invoker: Arc<Mutex<Option<Invoker<SpecctraMesadata>>>>,

    maybe_activity: Option<ActivityStepperWithStatus>,

    content_channel: (
        Sender<Result<SpecctraDesign, SpecctraLoadingError>>,
        Receiver<Result<SpecctraDesign, SpecctraLoadingError>>,
    ),
    history_channel: (
        Sender<std::io::Result<Result<History, serde_json::Error>>>,
        Receiver<std::io::Result<Result<History, serde_json::Error>>>,
    ),

    viewport: Viewport,

    menu_bar: MenuBar,
    status_bar: StatusBar,

    error_dialog: ErrorDialog,

    maybe_layers: Option<Layers>,
    maybe_design: Option<SpecctraDesign>,

    update_counter: f32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            config: Config::default(),
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
        let mut this = Self {
            translator: Translator::new(langid),
            ..Default::default()
        };
        // Load previous app state if one exists.
        if let Some(storage) = cc.storage {
            this.config = eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        }
        this
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
        if let Ok(data) = self.content_channel.1.try_recv() {
            match data {
                Ok(design) => match self.load_specctra_dsn(design) {
                    Ok(()) => {}
                    Err(err) => {
                        self.error_dialog
                            .push_error("tr-module-specctra-dsn-file-loader", err);
                    }
                },
                Err(SpecctraLoadingError::Parse(err)) => {
                    self.error_dialog.push_error(
                        "tr-module-specctra-dsn-file-loader",
                        format!(
                            "{}; {}",
                            self.translator
                                .text("tr-error-failed-to-parse-as-specctra-dsn"),
                            err
                        ),
                    );
                }
                Err(SpecctraLoadingError::Io(err)) => {
                    self.error_dialog.push_error(
                        "tr-module-specctra-dsn-file-loader",
                        format!(
                            "{}; {}",
                            self.translator.text("tr-error-unable-to-read-file"),
                            err
                        ),
                    );
                }
            }
        }

        if let Some(invoker) = self.arc_mutex_maybe_invoker.lock().unwrap().as_mut() {
            if let Ok(data) = self.history_channel.1.try_recv() {
                let tr = &self.translator;
                match data {
                    Ok(Ok(data)) => {
                        invoker.replay(data);
                    }
                    Ok(Err(err)) => {
                        self.error_dialog.push_error(
                            "tr-module-history-file-loader",
                            format!(
                                "{}; {}",
                                tr.text("tr-error-failed-to-parse-as-history-json"),
                                err
                            ),
                        );
                    }
                    Err(err) => {
                        self.error_dialog.push_error(
                            "tr-module-history-file-loader",
                            format!("{}; {}", tr.text("tr-error-unable-to-read-file"), err),
                        );
                    }
                }
            }

            if let Some(ref mut activity) = self.maybe_activity {
                return match activity.step(&mut ActivityContext {
                    interaction: InteractionContext {},
                    invoker,
                }) {
                    Ok(ActivityStatus::Running) => true,
                    Ok(ActivityStatus::Finished(..)) => false,
                    Err(err) => {
                        self.error_dialog
                            .push_error("tr-module-invoker", format!("{}", err));
                        false
                    }
                };
            }
        }

        false
    }

    fn load_specctra_dsn(&mut self, design: SpecctraDesign) -> Result<(), String> {
        let tr = &self.translator;
        let board = design.make_board();
        let overlay = Overlay::new(&board).map_err(|err| {
            format!(
                "{}; {}",
                tr.text("tr-error-unable-to-initialize-overlay"),
                err
            )
        })?;
        let layers = Layers::new(&board);
        let autorouter = Autorouter::new(board).map_err(|err| {
            format!(
                "{}; {}",
                tr.text("tr-error-unable-to-initialize-autorouter"),
                err
            )
        })?;
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
        eframe::set_value(storage, eframe::APP_KEY, &self.config);
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

pub async fn handle_file(file_handle: &rfd::FileHandle) -> io::Result<impl io::BufRead + io::Seek> {
    #[cfg(not(target_arch = "wasm32"))]
    let res = io::BufReader::new(std::fs::File::open(file_handle.path())?);

    #[cfg(target_arch = "wasm32")]
    let res = io::Cursor::new(file_handle.read().await);

    Ok(res)
}
