use topola::autorouter::invoker::{Execute, ExecuteWithStatus, InvokerStatus};

use crate::viewport::Viewport;

pub struct Bottom {}

impl Bottom {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        viewport: &Viewport,
        viewport_rect: egui::Rect,
        maybe_execute: &Option<ExecuteWithStatus>,
    ) {
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            let transform = egui::emath::RectTransform::from_to(viewport.from_rect, viewport_rect);
            let latest_pos = transform
                .inverse()
                .transform_pos(ctx.input(|i| i.pointer.latest_pos().unwrap_or_default()));

            let mut message = String::from("");

            if let Some(execute) = maybe_execute {
                if let Some(InvokerStatus::Finished(msg)) = execute.maybe_status() {
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
