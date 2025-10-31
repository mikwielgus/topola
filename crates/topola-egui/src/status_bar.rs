// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::ops::ControlFlow;

use topola::{interactor::activity::ActivityStepperWithStatus, stepper::EstimateLinearProgress};

use crate::{translator::Translator, viewport::Viewport};

pub struct StatusBar {}

impl StatusBar {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update<M>(
        &mut self,
        ctx: &egui::Context,
        _tr: &Translator,
        viewport: &Viewport,
        maybe_activity: Option<&ActivityStepperWithStatus<M>>,
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

            if let Some(activity) = maybe_activity {
                let linear_progress = activity.estimate_linear_progress();
                let value = linear_progress.value();
                let maximum = linear_progress.maximum();

                ui.add(
                    egui::ProgressBar::new((value / maximum) as f32).text(format!(
                        "{:.1} ({:.1}/{:.1})",
                        value / maximum * 100.0,
                        value,
                        maximum
                    )),
                );
            }
        });
    }
}
