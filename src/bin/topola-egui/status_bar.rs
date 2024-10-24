use std::ops::ControlFlow;

use topola::interactor::activity::ActivityStepperWithStatus;

use crate::{translator::Translator, viewport::Viewport};

pub struct StatusBar {}

impl StatusBar {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        tr: &Translator,
        viewport: &Viewport,
        maybe_activity: Option<&ActivityStepperWithStatus>,
    ) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            let latest_pos = viewport.transform.inverse()
                * ctx.input(|i| i.pointer.latest_pos().unwrap_or_default());

            let mut message = String::from("");

            if let Some(activity) = maybe_activity {
                if let Some(ControlFlow::Break(msg)) = activity.maybe_status() {
                    message = msg;
                }
            }

            ui.label(format!(
                "x: {} y: {} \t {}",
                latest_pos.x, -latest_pos.y, message
            ));
        });
    }
}
