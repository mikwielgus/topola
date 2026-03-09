// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT

use crate::{
    action::{Action, Trigger},
    translator::Translator,
};

pub struct Actions {
    pub file: FileActions,
}

impl Actions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            file: FileActions::new(tr),
        }
    }
}

pub struct FileActions {
    pub open_design: Trigger,
    pub export_session: Trigger,
    pub quit: Trigger,
}

impl FileActions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            open_design: Action::new(
                tr.text("tr-menu-file-open"),
                egui::Modifiers::CTRL,
                egui::Key::O,
            )
            .into_trigger(),
            export_session: Action::new(
                tr.text("tr-menu-file-export-session-file"),
                egui::Modifiers::CTRL,
                egui::Key::O,
            )
            .into_trigger(),
            quit: Action::new(
                tr.text("tr-menu-file-quit"),
                egui::Modifiers::CTRL,
                egui::Key::Q,
            )
            .into_trigger(),
        }
    }

    pub fn render_menu(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, have_workspace: bool) {
        self.open_design.button(ctx, ui);
        ui.add_enabled_ui(have_workspace, |ui| {
            self.export_session.button(ctx, ui);

            ui.separator();

            /*self.import_history.button(ctx, ui);
            self.export_history.button(ctx, ui);*/
        });

        ui.separator();

        // "Quit" button wouldn't work on a Web page.
        if !cfg!(target_arch = "wasm32") {
            self.quit.button(ctx, ui);
        }
    }
}
