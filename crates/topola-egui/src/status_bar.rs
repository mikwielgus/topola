// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::ops::ControlFlow;

use topola::{
    interactor::activity::ActivityStepperWithStatus,
    stepper::{EstimateProgress, GetTimeoutProgress},
};

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
                let progress = activity.estimate_progress();
                let value = progress.value();
                let maximum = progress.maximum();
                let ratio = *value as f32 / *maximum as f32;

                if let Some(trigger_progress) = activity.timeout_progress() {
                    ui.add(egui::ProgressBar::new(ratio).text(format!(
                        "{:.1}% ({:.1}/{:.1}) (timeout: {:.1}/{:.1}))",
                        ratio * 100.0,
                        value,
                        maximum,
                        trigger_progress.value(),
                        trigger_progress.maximum(),
                    )));
                } else {
                    ui.add(egui::ProgressBar::new(ratio).text(format!(
                        "{:.1}% ({:.1}/{:.1})",
                        ratio * 100.0,
                        value,
                        maximum
                    )));
                }

                let linear_subprogress = progress.subscale();
                let value = linear_subprogress.value();
                let maximum = linear_subprogress.maximum();
                let ratio = *value as f32 / *maximum as f32;

                ui.add(egui::ProgressBar::new(ratio).text(format!(
                    "{:.1}% ({:.1}/{:.1})",
                    ratio * 100.0,
                    value,
                    maximum
                )));
            }
        });
    }
}
