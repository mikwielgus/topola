use topola::autorouter::invoker::InvokerStatus;

use crate::{
    activity::{ActivityStatus, ActivityWithStatus},
    translator::Translator,
    viewport::Viewport,
};

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
        maybe_activity: &Option<ActivityWithStatus>,
    ) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            let latest_pos = viewport.transform.inverse()
                * ctx.input(|i| i.pointer.latest_pos().unwrap_or_default());

            let mut message = String::from("");

            if let Some(activity) = maybe_activity {
                if let Some(ActivityStatus::Finished(msg)) = activity.maybe_status() {
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
