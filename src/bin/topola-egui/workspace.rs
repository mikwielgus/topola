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
    error_dialog::ErrorDialog,
    interaction::InteractionContext,
    layers::Layers,
    overlay::Overlay,
    translator::Translator,
    viewport::Viewport,
};

/// A loaded design and associated structures
pub struct Workspace {
    pub design: SpecctraDesign,
    pub layers: Layers,
    pub overlay: Overlay,
    pub invoker: Invoker<SpecctraMesadata>,

    pub maybe_activity: Option<ActivityStepperWithStatus>,

    pub history_channel: (
        Sender<std::io::Result<Result<History, serde_json::Error>>>,
        Receiver<std::io::Result<Result<History, serde_json::Error>>>,
    ),
}

impl Workspace {
    pub fn new(design: SpecctraDesign, tr: &Translator) -> Result<Self, String> {
        let board = design.make_board();
        let overlay = Overlay::new(&board).map_err(|err| {
            format!(
                "{}; {}",
                tr.text("tr-error_unable-to-initialize-overlay"),
                err
            )
        })?;
        let layers = Layers::new(&board);
        let autorouter = Autorouter::new(board).map_err(|err| {
            format!(
                "{}; {}",
                tr.text("tr-error_unable-to-initialize-autorouter"),
                err
            )
        })?;
        Ok(Self {
            design,
            layers,
            overlay,
            invoker: Invoker::new(autorouter),
            maybe_activity: None,
            history_channel: channel(),
        })
    }

    pub fn update_state(&mut self, tr: &Translator, error_dialog: &mut ErrorDialog) -> bool {
        if let Ok(data) = self.history_channel.1.try_recv() {
            match data {
                Ok(Ok(data)) => {
                    self.invoker.replay(data);
                }
                Ok(Err(err)) => {
                    error_dialog.push_error(
                        "tr-module-history-file-loader",
                        format!(
                            "{}; {}",
                            tr.text("tr-error_failed-to-parse-as-history-json"),
                            err
                        ),
                    );
                }
                Err(err) => {
                    error_dialog.push_error(
                        "tr-module-history-file-loader",
                        format!("{}; {}", tr.text("tr-error_unable-to-read-file"), err),
                    );
                }
            }
        }

        if let Some(activity) = &mut self.maybe_activity {
            return match activity.step(&mut ActivityContext {
                interaction: InteractionContext {},
                invoker: &mut self.invoker,
            }) {
                Ok(ActivityStatus::Running) => true,
                Ok(ActivityStatus::Finished(..)) => false,
                Err(err) => {
                    error_dialog.push_error("tr-module-invoker", format!("{}", err));
                    false
                }
            };
        }
        false
    }

    pub fn update_layers(&mut self, ctx: &egui::Context) {
        self.layers.update(ctx, self.invoker.autorouter().board());
    }
}
