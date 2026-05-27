// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use egui::Pos2;
use topola::{InteractiveInput, SelectionCombineMode, SelectionInteractor, Vector2};

use crate::{display::Display, workspace::Workspace};

pub struct Viewport {
    pub scene_rect: egui::Rect,
    pub ref_scene_rect: egui::Rect,
    pub scheduled_zoom_to_fit: bool,
    selection_interactor: Option<SelectionInteractor>,
}

impl Viewport {
    pub fn new() -> Self {
        Self {
            scene_rect: egui::Rect::from_min_max(egui::pos2(-1.0, -1.0), egui::pos2(1.0, 1.0)),
            ref_scene_rect: egui::Rect::from_min_max(egui::pos2(-1.0, -1.0), egui::pos2(1.0, 1.0)),
            scheduled_zoom_to_fit: false,
            selection_interactor: None,
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, workspace: Option<&mut Workspace>) {
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
                        if let Some(ref workspace) = workspace {
                            let mut display = Display::new();
                            display.update(ctx, ui, &self, workspace);
                        }
                    })
                    .response;

                self.scene_rect = scene_rect;

                let scene_to_viewport =
                    Self::fit_to_rect_in_scene(viewport_rect, scene_rect, zoom_range.into());

                if let Some(workspace) = workspace {
                    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.selection_interactor = None;
                    }

                    let primary_pressed =
                        ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary));
                    let primary_down =
                        ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
                    let primary_released =
                        ctx.input(|i| i.pointer.button_released(egui::PointerButton::Primary));
                    let mut maybe_pointer_on_scene: Option<Vector2<i64>> = None;

                    if let Some(pointer_viewport_pos) = ctx.input(|i| i.pointer.interact_pos()) {
                        let pointer_on_scene_pos =
                            scene_to_viewport.inverse() * pointer_viewport_pos;
                        let pointer_on_scene = Vector2::new(
                            pointer_on_scene_pos.x as i64,
                            pointer_on_scene_pos.y as i64,
                        );
                        maybe_pointer_on_scene = Some(pointer_on_scene);

                        if primary_pressed && response.hovered() {
                            self.selection_interactor = Some(SelectionInteractor::new(
                                pointer_on_scene,
                                workspace.selection.clone(),
                                SelectionCombineMode::Replace,
                            ));
                        }

                        if let Some(interactor) = self.selection_interactor.as_mut() {
                            if primary_down {
                                let _ = interactor.update(
                                    workspace.autorouter.router().navmesher_board().board(),
                                    workspace.appearance_panel.active,
                                    InteractiveInput::new(pointer_on_scene, false, false),
                                );
                            }
                        }
                    }

                    if primary_released {
                        if let Some(mut interactor) = self.selection_interactor.take() {
                            let pointer_for_scene =
                                maybe_pointer_on_scene.unwrap_or(*interactor.origin());
                            let _ = interactor.update(
                                workspace.autorouter.router().navmesher_board().board(),
                                workspace.appearance_panel.active,
                                InteractiveInput::new(pointer_for_scene, true, false),
                            );

                            workspace.selection = interactor.selection().clone();
                        }
                    }

                    self.zoom_to_fit_if_scheduled(workspace);
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
        // Compute the scale factor to fit the bounding rectangle into the available screen size:
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

    fn zoom_to_fit_if_scheduled(&mut self, workspace: &Workspace) {
        if self.scheduled_zoom_to_fit {
            self.scene_rect = Self::boundary_bounding_box(workspace);
            self.ref_scene_rect = self.scene_rect.clone();
        }

        self.scheduled_zoom_to_fit = false;
    }

    fn boundary_bounding_box(workspace: &Workspace) -> egui::Rect {
        let first = workspace
            .autorouter
            .router()
            .navmesher_board()
            .board()
            .layout()
            .boundary()[0];

        let mut min_x = first[0];
        let mut max_x = first[0];
        let mut min_y = first[1];
        let mut max_y = first[1];

        for point in workspace
            .autorouter
            .router()
            .navmesher_board()
            .board()
            .layout()
            .boundary()[1..]
            .iter()
        {
            if point[0] < min_x {
                min_x = point[0];
            }
            if point[0] > max_x {
                max_x = point[0];
            }
            if point[1] < min_y {
                min_y = point[1];
            }
            if point[1] > max_y {
                max_y = point[1];
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
