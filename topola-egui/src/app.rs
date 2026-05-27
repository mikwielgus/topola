// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::sync::mpsc::{Receiver, Sender, channel};

use specctra::{error::ParseErrorContext, structure::DsnFile};
use topola::Board;
use unic_langid::langid;

use crate::{
    menu_bar::MenuBar, translator::Translator, viewport::Viewport, workspace::GuiWorkspace,
};

pub struct App {
    translator: Translator,

    content_channel: (
        Sender<Result<DsnFile, ParseErrorContext>>,
        Receiver<Result<DsnFile, ParseErrorContext>>,
    ),

    menu_bar: MenuBar,
    viewport: Viewport,
    workspace: Option<GuiWorkspace>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            translator: Translator::new(langid!("en-US")),
            content_channel: channel(),
            menu_bar: MenuBar::new(),
            viewport: Viewport::new(),
            workspace: None,
        }
    }
}

impl App {
    /// Called once on start.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Restore the persistent part of the app's state from its previous run
        // if there is one.
        if let Some(storage) = cc.storage {
            //eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
            Default::default()
        } else {
            Default::default()
        }
    }

    fn update_state(&mut self) {
        if let Ok(data) = self.content_channel.1.try_recv() {
            self.workspace = Some(GuiWorkspace::new(
                Board::from_specctra(data.unwrap()),
                &self.translator,
            ));
            self.viewport.scheduled_zoom_to_fit = true;
        }
    }

    /// Update the title displayed on the application window's frame to show the
    /// currently opened file, if any, and other possible information about the
    /// application state.
    #[cfg(not(target_arch = "wasm32"))]
    fn update_title(&mut self, ctx: &egui::Context) {
        /*if let Some(workspace) = &self.maybe_workspace {
            if let Some(filename) = Path::new(workspace.design.get_name())
                .file_name()
                .and_then(|n| n.to_str())
            {
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(filename.to_string()));
                // TODO: Also show file's dirty bit.
            }
        }*/
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
        //eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI has to be repainted.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.menu_bar
            .update(ctx, &mut self.translator, self.content_channel.0.clone());

        self.update_state();

        if let Some(ref mut workspace) = self.workspace {
            workspace.update_appearance_panel(ctx);
        }

        self.viewport.update(ctx, self.workspace.as_mut());

        self.update_locale();
        self.update_title(ctx);
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

pub async fn handle_file(
    file_handle: &rfd::FileHandle,
) -> std::io::Result<impl std::io::BufRead + std::io::Seek> {
    #[cfg(not(target_arch = "wasm32"))]
    let res = std::io::BufReader::new(std::fs::File::open(file_handle.path())?);

    #[cfg(target_arch = "wasm32")]
    let res = std::io::Cursor::new(file_handle.read().await);

    Ok(res)
}
