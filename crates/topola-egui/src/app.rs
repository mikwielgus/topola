// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{
    future::Future,
    io,
    path::Path,
    sync::mpsc::{channel, Receiver, Sender},
};
use unic_langid::{langid, LanguageIdentifier};

use topola::specctra::{design::SpecctraDesign, ParseErrorContext as SpecctraLoadingError};

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
        // Restore the persistent part of the app's state from its previous run
        // if there is one.
        if let Some(storage) = cc.storage {
            this.config = eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        }
        this
    }

    /// Advance the app's state by a single step.
    fn update_state(&mut self) {
        // If a new design has been loaded from a file, create a new workspace
        // with the design. Or handle the error if there was a failure to do so.
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
                Err(err) => match &err.error {
                    topola::specctra::ParseError::Io(err) => {
                        self.error_dialog.push_error(
                            "tr-module-specctra-dsn-file-loader",
                            format!(
                                "{}; {}",
                                self.translator.text("tr-error-unable-to-read-file"),
                                err
                            ),
                        );
                    }
                    _ => {
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
                },
            }
        }
    }

    /// Update the title displayed on the application window's frame to show the
    /// currently opened file, if any, and other possible information about the
    /// application state.
    #[cfg(not(target_arch = "wasm32"))]
    fn update_title(&mut self, ctx: &egui::Context) {
        if let Some(workspace) = &self.maybe_workspace {
            if let Some(filename) = Path::new(workspace.design.get_name())
                .file_name()
                .and_then(|n| n.to_str())
            {
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(filename.to_string()));
                // TODO: Also show file's dirty bit.
            }
        }
    }

    /// Update the title displayed on the application window's frame to show the
    /// currently opened file, if any, and other possible information about the
    /// application state.
    #[cfg(target_arch = "wasm32")]
    fn update_title(&mut self, ctx: &egui::Context) {
        if let Some(workspace) = &self.maybe_workspace {
            if let Some(filename) = Path::new(workspace.design.get_name())
                .file_name()
                .and_then(|n| n.to_str())
            {
                let document = eframe::web_sys::window()
                    .expect("No window")
                    .document()
                    .expect("No document");

                document.set_title(filename);
                // TODO: Also show file's dirty bit.
            }
        }
    }

    /// Handle a possible locale change.
    #[cfg(not(target_arch = "wasm32"))]
    fn update_locale(&mut self) {
        // I don't know any equivalent of changing the lang property in desktop.
    }

    /// Handle a possible locale change.
    #[cfg(target_arch = "wasm32")]
    fn update_locale(&mut self) {
        let document_element = eframe::web_sys::window()
            .expect("No window")
            .document()
            .expect("No document")
            .document_element()
            .expect("No document element");

        document_element.set_attribute("lang", &self.translator.langid().to_string());
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
            &mut self.translator,
            self.content_channel.0.clone(),
            &mut self.error_dialog,
            &mut self.viewport,
            self.maybe_workspace.as_mut(),
        );

        self.update_state();

        if self.menu_bar.show_appearance_panel {
            if let Some(workspace) = &mut self.maybe_workspace {
                workspace
                    .update_appearance_panel(ctx, &mut self.menu_bar.multilayer_autoroute_options);
            }
        }

        self.error_dialog.update(ctx, &self.translator);

        let _viewport_rect = self.viewport.update(
            &self.config,
            ctx,
            &self.translator,
            &self.menu_bar,
            &mut self.error_dialog,
            self.maybe_workspace.as_mut(),
        );

        self.status_bar.update(
            ctx,
            &self.translator,
            &self.viewport,
            self.maybe_workspace
                .as_ref()
                .and_then(|w| w.interactor.maybe_activity().as_ref()),
        );

        self.update_locale();
        self.update_title(ctx);

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
