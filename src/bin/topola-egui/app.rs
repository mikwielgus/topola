use std::{
    future::Future,
    io,
    sync::mpsc::{channel, Receiver, Sender},
};
use unic_langid::{langid, LanguageIdentifier};

use topola::specctra::design::{LoadingError as SpecctraLoadingError, SpecctraDesign};

use crate::{
    config::Config, error_dialog::ErrorDialog, menu_bar::MenuBar, status_bar::StatusBar,
    translator::Translator, viewport::Viewport, workspace::Workspace,
};

pub struct App {
    config: Config,
    translator: Translator,

    content_channel: (
        Sender<Result<SpecctraDesign, SpecctraLoadingError>>,
        Receiver<Result<SpecctraDesign, SpecctraLoadingError>>,
    ),

    viewport: Viewport,
    menu_bar: MenuBar,
    status_bar: StatusBar,
    error_dialog: ErrorDialog,

    maybe_workspace: Option<Workspace>,

    update_counter: f32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            config: Config::default(),
            translator: Translator::new(langid!("en-US")),
            content_channel: channel(),
            viewport: Viewport::new(),
            menu_bar: MenuBar::new(),
            status_bar: StatusBar::new(),
            error_dialog: ErrorDialog::new(),
            maybe_workspace: None,
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
                Ok(design) => match Workspace::new(design, &self.translator) {
                    Ok(ws) => {
                        self.maybe_workspace = Some(ws);
                        self.viewport.scheduled_zoom_to_fit = true;
                    }
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

        if let Some(workspace) = &mut self.maybe_workspace {
            return workspace.update_state(&self.translator, &mut self.error_dialog);
        }
        false
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
            &mut self.viewport,
            self.maybe_workspace.as_mut(),
        );

        self.advance_state_by_dt(ctx.input(|i| i.stable_dt));

        self.status_bar.update(
            ctx,
            &self.translator,
            &self.viewport,
            self.maybe_workspace
                .as_ref()
                .and_then(|w| w.maybe_activity.as_ref()),
        );

        if self.menu_bar.show_layer_manager {
            if let Some(workspace) = &mut self.maybe_workspace {
                workspace.update_layers(ctx);
            }
        }

        self.error_dialog.update(ctx, &self.translator);

        let _viewport_rect =
            self.viewport
                .update(ctx, &self.menu_bar, self.maybe_workspace.as_mut());

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
