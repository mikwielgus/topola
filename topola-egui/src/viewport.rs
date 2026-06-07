// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use egui::Pos2;

use crate::{controller::Controller, display::Display, menu_bar::MenuBar, translator::Translator};

pub struct Viewport {
    pub scene_rect: egui::Rect,
    pub ref_scene_rect: egui::Rect,
    pub scheduled_zoom_to_fit: bool,
}

impl Viewport {
    pub fn new() -> Self {
        Self {
            scene_rect: egui::Rect::from_min_max(egui::pos2(-1.0, -1.0), egui::pos2(1.0, 1.0)),
            ref_scene_rect: egui::Rect::from_min_max(egui::pos2(-1.0, -1.0), egui::pos2(1.0, 1.0)),
            scheduled_zoom_to_fit: false,
        }
    }

    pub fn update(
        &mut self,
        tr: &Translator,
        ctx: &egui::Context,
        menu_bar: &MenuBar,
        controller: Option<&mut Controller>,
    ) {
        crate::profile_function!();

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                ui.ctx().request_repaint();

                let zoom_range = 0.00001..=10000.0;

                let viewport_rect = ui.available_rect_before_wrap();
                let mut scene_rect = self.scene_rect.clone();

                let response = egui::Scene::new()
                    .zoom_range(zoom_range.clone())
                    .drag_pan_buttons(egui::DragPanButtons::MIDDLE)
                    .show(ui, &mut scene_rect, |ui| {
                        if let Some(ref controller) = controller {
                            let mut display = Display::new();
                            display.update(ctx, ui, &self, controller);
                        }
                    })
                    .response;

                self.scene_rect = scene_rect;

                let scene_to_viewport =
                    Self::fit_to_rect_in_scene(viewport_rect, scene_rect, zoom_range.into());

                if let Some(controller) = controller {
                    controller.advance_state_by_dt(
                        tr,
                        menu_bar.fix_step_rate.then_some(menu_bar.step_rate),
                        ctx.input(|i| {
                            if i.stable_dt <= i.predicted_dt {
                                i.stable_dt
                            } else {
                                i.predicted_dt
                            }
                        }) as f64,
                    );

                    controller.handle_input(
                        tr,
                        ctx,
                        menu_bar.fix_step_rate.then_some(menu_bar.step_rate),
                        scene_to_viewport,
                        response.hovered(),
                        ui,
                    );

                    self.zoom_to_fit_if_scheduled(controller);
                }
            })
        });
    }

    /// Copied from egui/containers/scene.rs and modified.
    ///
    /// Creates a transformation that fits a given scene rectangle into the available screen size.
    ///
    /// The resulting visual scene bounds can be larger, due to letterboxing.
    ///
    /// Returns the transformation from `scene` to `global` coordinates.
    fn fit_to_rect_in_scene(
        rect_in_viewport: egui::Rect,
        rect_in_scene: egui::Rect,
        zoom_range: egui::Rangef,
    ) -> egui::emath::TSTransform {
        // Compute the scale factor to fit the bounding rect into the available
        // screen size:
        let scale = rect_in_viewport.size() / rect_in_scene.size();

        // Use the smaller of the two scales to ensure the whole rectangle fits on the screen:
        let scale = scale.min_elem();

        // Clamp scale to what is allowed
        let scale = zoom_range.clamp(scale);

        // Compute the translation to center the bounding rect in the screen:
        let center_in_global = rect_in_viewport.center().to_vec2();
        let center_scene = rect_in_scene.center().to_vec2();

        // Set the transformation to scale and then translate to center.
        egui::emath::TSTransform::from_translation(center_in_global - scale * center_scene)
            * egui::emath::TSTransform::from_scaling(scale)
    }

    fn zoom_to_fit_if_scheduled(&mut self, controller: &Controller) {
        if self.scheduled_zoom_to_fit {
            self.scene_rect = Self::boundary_bounding_box(controller);
            self.ref_scene_rect = self.scene_rect.clone();
        }

        self.scheduled_zoom_to_fit = false;
    }

    fn boundary_bounding_box(controller: &Controller) -> egui::Rect {
        let first = controller.workspace.board().layout().boundary()[0];

        let mut min_x = first.x;
        let mut max_x = first.x;
        let mut min_y = first.y;
        let mut max_y = first.y;

        for point in controller.workspace.board().layout().boundary()[1..].iter() {
            if point.x < min_x {
                min_x = point.x;
            }
            if point.x > max_x {
                max_x = point.x;
            }
            if point.y < min_y {
                min_y = point.y;
            }
            if point.y > max_y {
                max_y = point.y;
            }
        }

        egui::Rect::from_min_max(
            Pos2::new(min_x as f32, min_y as f32),
            Pos2::new(max_x as f32, max_y as f32),
        )
        .scale_from_center(1.05)
    }

    pub fn scale_factor(&self) -> f32 {
        self.ref_scene_rect.width() / self.scene_rect.width()
    }
}
