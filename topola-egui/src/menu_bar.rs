// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::sync::mpsc::Sender;

use egui::{
    PopupCloseBehavior,
    containers::menu::{MenuButton, MenuConfig},
};
use serde::{Deserialize, Serialize};
use specctra::{
    error::{ParseError, ParseErrorContext},
    read::ListTokenizer,
    structure::DsnFile,
};
use topola::AutoplacerSchedule;

use crate::{
    actions::Actions,
    app::{execute, handle_file},
    controller::Controller,
    translator::Translator,
};

#[derive(Deserialize, Serialize)]
pub struct MenuBar {
    pub fix_step_rate: bool,
    pub step_rate: f64,
}

impl MenuBar {
    pub fn new() -> Self {
        Self {
            fix_step_rate: false,
            step_rate: 1.0,
        }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        tr: &mut Translator,
        content_sender: Sender<Result<DsnFile, ParseErrorContext>>,
        controller: Option<&mut Controller>,
    ) {
        let mut actions = Actions::new(tr);

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    actions.file.render_menu(ctx, ui, controller.is_some());
                });

                ui.separator();

                actions.run.render_menu(ctx, ui, controller.is_some());

                ui.separator();

                MenuButton::new(tr.text("tr-menu-debug"))
                    .config(
                        MenuConfig::default()
                            .close_behavior(PopupCloseBehavior::CloseOnClickOutside),
                    )
                    .ui(ui, |ui| {
                        actions.debug.render_menu(ctx, ui, self);

                        ui.add_enabled_ui(self.fix_step_rate, |ui| {
                            ui.label(tr.text("tr-menu-debug-step-rate"));
                            ui.add(
                                egui::widgets::Slider::new(&mut self.step_rate, 0.1..=1000.0)
                                    .suffix(format!(
                                        " {}",
                                        tr.text("tr-menu-debug-step-rate-unit")
                                    )),
                            );
                        });
                    });

                ui.separator();

                egui::widgets::global_theme_preference_switch(ui);
            });

            if actions.file.open_design.consume_key_triggered(ctx, ui) {
                // NOTE: On Linux, this requires Zenity to be installed on your system.
                let ctx = ctx.clone();
                let task = rfd::AsyncFileDialog::new()
                    .add_filter(tr.text("tr-menu-open-specctra-design-file"), &["dsn"])
                    .pick_file();

                execute(async move {
                    if let Some(file_handle) = task.await {
                        let data = handle_file(&file_handle)
                            .await
                            .map_err(|e| ParseError::from(e).add_context((0, 0)))
                            .and_then(|reader| ListTokenizer::new(reader).read_value::<DsnFile>());
                        content_sender.send(data);

                        ctx.request_repaint();
                    }
                });
            }

            if actions.run.autoplace.consume_key_triggered(ctx, ui) {
                if let Some(controller) = controller {
                    controller.master_interactor.autoplace(
                        controller.workspace.board_mut(),
                        AutoplacerSchedule {
                            initial_temperature: 1000.0,
                            temperature_common_ratio: 0.95,
                            initial_std_dev: 1000.0,
                            std_dev_common_ratio: 0.995,
                            max_steps: 200,
                        },
                    );
                }
            }
        });
    }
}
