// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT

use egui::{Context, Ui};

use crate::{
    action::{Action, Switch, Trigger},
    menu_bar::MenuBar,
    translator::Translator,
};

pub struct Actions {
    pub file: FileActions,
    pub edit: EditActions,
    pub run: RunActions,
    pub debug: DebugActions,
}

impl Actions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            file: FileActions::new(tr),
            edit: EditActions::new(tr),
            run: RunActions::new(tr),
            debug: DebugActions::new(tr),
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

pub struct EditActions {
    pub undo: Trigger,
    pub redo: Trigger,
}

impl EditActions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            undo: Action::new(
                tr.text("tr-menu-edit-undo"),
                egui::Modifiers::CTRL,
                egui::Key::Z,
            )
            .into_trigger(),
            redo: Action::new(
                tr.text("tr-menu-edit-redo"),
                egui::Modifiers::CTRL,
                egui::Key::Y,
            )
            .into_trigger(),
        }
    }

    pub fn render_menu(
        &mut self,
        ctx: &Context,
        ui: &mut Ui,
        have_workspace: bool,
        can_undo: bool,
        can_redo: bool,
    ) {
        ui.add_enabled_ui(have_workspace, |ui| {
            ui.add_enabled_ui(can_undo, |ui| {
                self.undo.button(ctx, ui);
            });
            ui.add_enabled_ui(can_redo, |ui| {
                self.redo.button(ctx, ui);
            });
        });
    }
}

pub struct RunActions {
    pub autoplace: Trigger,
}

impl RunActions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            autoplace: Action::new(
                tr.text("tr-menu-route-autoplace"),
                egui::Modifiers::NONE,
                egui::Key::Space,
            )
            .into_trigger(),
        }
    }

    pub fn render_menu(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, have_workspace: bool) {
        ui.add_enabled_ui(have_workspace, |ui| {
            self.autoplace.button(ctx, ui);
        });
    }
}

pub struct DebugActions {
    pub fix_step_rate: Switch,
    pub show_repulsions: Switch,
    pub show_attractions: Switch,
    pub show_retentions: Switch,
    pub show_bboxes: Switch,
    pub show_navmeshes: Switch,
}

impl DebugActions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            fix_step_rate: Action::new_keyless(tr.text("tr-menu-debug-fix-step-rate"))
                .into_switch(),
            show_repulsions: Action::new_keyless(tr.text("tr-menu-debug-show-repulsions"))
                .into_switch(),
            show_attractions: Action::new_keyless(tr.text("tr-menu-debug-show-attractions"))
                .into_switch(),
            show_retentions: Action::new_keyless(tr.text("tr-menu-debug-show-retentions"))
                .into_switch(),
            show_bboxes: Action::new_keyless(tr.text("tr-menu-debug-show-bboxes")).into_switch(),
            show_navmeshes: Action::new_keyless(tr.text("tr-menu-debug-show-navmesh"))
                .into_switch(),
        }
    }

    pub fn render_menu(&mut self, _ctx: &Context, ui: &mut Ui, menu_bar: &mut MenuBar) {
        self.fix_step_rate.checkbox(ui, &mut menu_bar.fix_step_rate);

        ui.separator();

        self.show_repulsions
            .checkbox(ui, &mut menu_bar.show_repulsions);
        self.show_attractions
            .checkbox(ui, &mut menu_bar.show_attractions);
        self.show_retentions
            .checkbox(ui, &mut menu_bar.show_retentions);
        self.show_bboxes.checkbox(ui, &mut menu_bar.show_bboxes);
        self.show_navmeshes
            .checkbox(ui, &mut menu_bar.show_navmeshes);
    }
}
