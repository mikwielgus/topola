// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use egui::Pos2;
use topola::{Interactor, MasterInteractor, Vector2, Workspace};

use crate::{display::Display, menu_bar::MenuBar, translator::Translator, workspace::GuiWorkspace};

pub struct Viewport {
    pub scene_rect: egui::Rect,
    pub ref_scene_rect: egui::Rect,
    pub scheduled_zoom_to_fit: bool,
    master_interactor: Option<MasterInteractor>,
}

impl Viewport {
    pub fn new() -> Self {
        Self {
            scene_rect: egui::Rect::from_min_max(egui::pos2(-1.0, -1.0), egui::pos2(1.0, 1.0)),
            ref_scene_rect: egui::Rect::from_min_max(egui::pos2(-1.0, -1.0), egui::pos2(1.0, 1.0)),
            scheduled_zoom_to_fit: false,
            master_interactor: None,
        }
    }

    pub fn update(
        &mut self,
        tr: &Translator,
        ctx: &egui::Context,
        menu_bar: &MenuBar,
        workspace: Option<&mut GuiWorkspace>,
    ) {
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
                    workspace.advance_state_by_dt(
                        tr,
                        menu_bar.fix_step_rate.then_some(menu_bar.step_rate),
                        ctx.input(|i| {
                            if i.stable_dt <= i.predicted_dt {
                                i.stable_dt
                            } else {
                                // Clamp dt to egui's predicted dt to
                                // additionally safeguard against stuttering.
                                i.predicted_dt
                            }
                        }) as f64,
                    );

                    let escape_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));
                    if escape_pressed {
                        if let Some(interactor) = self.master_interactor.as_mut() {
                            let board = match &mut workspace.workspace {
                                Workspace::Board(workspace) => &mut workspace.board,
                                Workspace::Autorouter(_) => panic!("expected board workspace"),
                            };
                            interactor.abort(board);
                            *workspace.workspace.selection_mut() = interactor.selection().clone();
                        }
                        self.master_interactor = None;
                    }

                    let primary_pressed =
                        ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary));
                    let primary_down =
                        ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
                    let primary_released =
                        ctx.input(|i| i.pointer.button_released(egui::PointerButton::Primary));

                    let delete_pressed = ctx.input(|i| i.key_pressed(egui::Key::Delete));
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
                            self.master_interactor = Some(MasterInteractor::new(
                                workspace.workspace.selection().clone(),
                            ));
                        }

                        if let Some(interactor) = self.master_interactor.as_mut() {
                            if primary_down {
                                let board = match &mut workspace.workspace {
                                    Workspace::Board(workspace) => &mut workspace.board,
                                    Workspace::Autorouter(_) => panic!("expected board workspace"),
                                };
                                interactor.hold(
                                    board,
                                    workspace.appearance_panel.active,
                                    pointer_on_scene,
                                );

                                if let Some(select_interactor) =
                                    interactor.select_interactor().as_ref()
                                {
                                    let origin = *select_interactor.origin();
                                    let drag_rect_scene = egui::Rect::from_min_max(
                                        egui::pos2(
                                            origin.x.min(pointer_on_scene.x) as f32,
                                            origin.y.min(pointer_on_scene.y) as f32,
                                        ),
                                        egui::pos2(
                                            origin.x.max(pointer_on_scene.x) as f32,
                                            origin.y.max(pointer_on_scene.y) as f32,
                                        ),
                                    );

                                    let drag_rect_on_viewport = egui::Rect::from_min_max(
                                        scene_to_viewport * drag_rect_scene.min,
                                        scene_to_viewport * drag_rect_scene.max,
                                    );
                                    let boundary_color = if pointer_on_scene.x >= origin.x {
                                        egui::Color32::YELLOW
                                    } else {
                                        egui::Color32::from_rgb(80, 160, 255)
                                    };

                                    ui.painter().rect(
                                        drag_rect_on_viewport,
                                        egui::CornerRadius::ZERO,
                                        egui::Color32::from_rgba_unmultiplied(80, 160, 255, 48),
                                        egui::Stroke::new(1.5, boundary_color),
                                        egui::StrokeKind::Outside,
                                    );
                                }
                            }
                        }
                    }

                    if primary_released {
                        if let Some(mut interactor) = self.master_interactor.take() {
                            let pointer_for_scene = maybe_pointer_on_scene.unwrap_or_else(|| {
                                interactor
                                    .select_interactor()
                                    .as_ref()
                                    .map(|select_interactor| *select_interactor.origin())
                                    .unwrap_or(Vector2::new(0, 0))
                            });
                            let board = match &mut workspace.workspace {
                                Workspace::Board(workspace) => &mut workspace.board,
                                Workspace::Autorouter(_) => panic!("expected board workspace"),
                            };
                            interactor.release(
                                board,
                                workspace.appearance_panel.active,
                                pointer_for_scene,
                            );

                            *workspace.workspace.selection_mut() = interactor.selection().clone();
                        }
                    }

                    if delete_pressed {
                        let mut interactor =
                            MasterInteractor::new(workspace.workspace.selection().clone());
                        let board = match &mut workspace.workspace {
                            Workspace::Board(workspace) => &mut workspace.board,
                            Workspace::Autorouter(_) => panic!("expected board workspace"),
                        };
                        interactor.delete(board);
                        *workspace.workspace.selection_mut() = interactor.selection().clone();
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

    fn zoom_to_fit_if_scheduled(&mut self, workspace: &GuiWorkspace) {
        if self.scheduled_zoom_to_fit {
            self.scene_rect = Self::boundary_bounding_box(workspace);
            self.ref_scene_rect = self.scene_rect.clone();
        }

        self.scheduled_zoom_to_fit = false;
    }

    fn boundary_bounding_box(workspace: &GuiWorkspace) -> egui::Rect {
        let first = workspace.workspace.board().layout().boundary()[0];

        let mut min_x = first[0];
        let mut max_x = first[0];
        let mut min_y = first[1];
        let mut max_y = first[1];

        for point in workspace.workspace.board().layout().boundary()[1..].iter() {
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
