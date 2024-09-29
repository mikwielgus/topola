use topola::autorouter::invoker::InvokerStatus;

use crate::{activity::ActivityWithStatus, translator::Translator, viewport::Viewport};

pub struct Bottom {}

impl Bottom {
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
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            let latest_pos = viewport.transform.inverse()
                * ctx.input(|i| i.pointer.latest_pos().unwrap_or_default());

            let mut message = String::from("");

            if let Some(activity) = maybe_activity {
                if let Some(InvokerStatus::Finished(msg)) = activity.maybe_status() {
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
