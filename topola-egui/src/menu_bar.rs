// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::sync::mpsc::Sender;

use serde::{Deserialize, Serialize};
use specctra::{
    error::{ParseError, ParseErrorContext},
    read::ListTokenizer,
    structure::DsnFile,
};

use crate::{
    actions::Actions,
    app::{execute, handle_file},
    translator::Translator,
};

#[derive(Deserialize, Serialize)]
pub struct MenuBar {
    step_rate: f32,
}

impl MenuBar {
    pub fn new() -> Self {
        Self { step_rate: 1.0 }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        tr: &mut Translator,
        content_sender: Sender<Result<DsnFile, ParseErrorContext>>,
    ) {
        let mut actions = Actions::new(tr);

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    actions.file.render_menu(ctx, ui, false);
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
        });
    }
}
