use std::{
    ops::ControlFlow,
    sync::mpsc::{channel, Receiver, Sender},
};

use topola::{
    autorouter::{history::History, invoker::Invoker, Autorouter},
    interactor::{
        activity::{ActivityContext, ActivityStepperWithStatus},
        interaction::InteractionContext,
        Interactor,
    },
    specctra::{design::SpecctraDesign, mesadata::SpecctraMesadata},
    stepper::Step,
};

use crate::{error_dialog::ErrorDialog, layers::Layers, overlay::Overlay, translator::Translator};

/// A loaded design and associated structures
pub struct Workspace {
    pub design: SpecctraDesign,
    pub layers: Layers,
    pub overlay: Overlay,
    pub interactor: Interactor<SpecctraMesadata>,

    pub history_channel: (
        Sender<std::io::Result<Result<History, serde_json::Error>>>,
        Receiver<std::io::Result<Result<History, serde_json::Error>>>,
    ),
}

impl Workspace {
    pub fn new(design: SpecctraDesign, tr: &Translator) -> Result<Self, String> {
        let board = design.make_board();
        let layers = Layers::new(&board);
        let overlay = Overlay::new(&board).map_err(|err| {
            format!(
                "{}; {}",
                tr.text("tr-error-unable-to-initialize-overlay"),
                err
            )
        })?;
        Ok(Self {
            design,
            layers,
            overlay,
            interactor: Interactor::new(board).map_err(|err| {
                format!(
                    "{}; {}",
                    tr.text("tr-error_unable-to-initialize-overlay"),
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

        match self.interactor.update() {
            ControlFlow::Continue(()) => ControlFlow::Continue(()),
            ControlFlow::Break(Ok(())) => ControlFlow::Break(()),
            ControlFlow::Break(Err(err)) => {
                error_dialog.push_error("tr-module-invoker", format!("{}", err));
                ControlFlow::Break(())
            }
        }
    }

    pub fn update_layers(&mut self, ctx: &egui::Context) {
        self.layers
            .update(ctx, self.interactor.invoker().autorouter().board());
    }
}
