// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{
    ops::ControlFlow,
    sync::mpsc::{channel, Receiver, Sender},
};

use topola::{
    autorouter::history::History,
    interactor::{activity::InteractiveInput, Interactor},
    layout::LayoutEdit,
    specctra::{design::SpecctraDesign, mesadata::SpecctraMesadata},
};

use crate::{
    appearance_panel::AppearancePanel, error_dialog::ErrorDialog, overlay::Overlay,
    translator::Translator,
};

/// A loaded design and associated structures.
pub struct Workspace {
    pub design: SpecctraDesign,
    pub appearance_panel: AppearancePanel,
    pub overlay: Overlay,
    pub interactor: Interactor<SpecctraMesadata>,

    pub history_channel: (
        Sender<std::io::Result<Result<History, serde_json::Error>>>,
        Receiver<std::io::Result<Result<History, serde_json::Error>>>,
    ),
}

impl Workspace {
    pub fn new(design: SpecctraDesign, tr: &Translator) -> Result<Self, String> {
        let board = design.make_board(&mut LayoutEdit::new());
        let appearance_panel = AppearancePanel::new(&board);
        let overlay = Overlay::new(&board).map_err(|err| {
            format!(
                "{}; {}",
                tr.text("tr-error-unable-to-initialize-overlay"),
                err
            )
        })?;
        Ok(Self {
            design,
            appearance_panel,
            overlay,
            interactor: Interactor::new(board).map_err(|err| {
                format!(
                    "{}; {}",
                    tr.text("tr-error-unable-to-initialize-overlay"),
                    err
                )
            })?,
            history_channel: channel(),
        })
    }

    pub fn update_state(
        &mut self,
        tr: &Translator,
        error_dialog: &mut ErrorDialog,
        interactive_input: &InteractiveInput,
    ) -> ControlFlow<()> {
        if let Ok(data) = self.history_channel.1.try_recv() {
            match data {
                Ok(Ok(data)) => {
                    self.interactor.replay(data);
                }
                Ok(Err(err)) => {
                    error_dialog.push_error(
                        "tr-module-history-file-loader",
                        format!(
                            "{}; {}",
                            tr.text("tr-error-failed-to-parse-as-history-json"),
                            err
                        ),
                    );
                }
                Err(err) => {
                    error_dialog.push_error(
                        "tr-module-history-file-loader",
                        format!("{}; {}", tr.text("tr-error-unable-to-read-file"), err),
                    );
                }
            }
        }

        match self.interactor.update(interactive_input) {
            ControlFlow::Continue(()) => ControlFlow::Continue(()),
            ControlFlow::Break(Ok(())) => ControlFlow::Break(()),
            ControlFlow::Break(Err(err)) => {
                error_dialog.push_error("tr-module-invoker", format!("{}", err));
                ControlFlow::Break(())
            }
        }
    }

    pub fn update_appearance_panel(&mut self, ctx: &egui::Context) {
        self.appearance_panel
            .update(ctx, self.interactor.invoker().autorouter().board());
    }
}
