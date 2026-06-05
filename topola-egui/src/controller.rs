// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{ops::ControlFlow, time::Instant};

use topola::{Interactor, MasterInteractor, Vector2, Workspace, board::Board};

use crate::{layers_panel::LayersPanel, translator::Translator};

pub struct Controller {
    pub workspace: Workspace,
    pub appearance_panel: LayersPanel,
    pub master_interactor: MasterInteractor,
    dt_accum: f64,
}

impl Controller {
    pub fn new(board: Board, tr: &Translator) -> Self {
        let appearance_panel = LayersPanel::new(&board);
        let workspace = Workspace::new_board(board);

        Self {
            master_interactor: MasterInteractor::new(workspace.selection().clone()),
            workspace,
            appearance_panel,
            dt_accum: 0.0,
        }
    }

    pub fn advance_state_by_dt(
        &mut self,
        tr: &Translator,
        step_rate: Option<f64>,
        dt: f64,
    ) -> bool {
        let instant = Instant::now();

        if step_rate.is_some() {
            self.dt_accum += dt;
        }

        while step_rate.is_none_or(|step_rate| self.dt_accum >= 1.0 / step_rate) {
            if let Some(step_rate) = step_rate {
                self.dt_accum -= 1.0 / step_rate;
            }

            if let ControlFlow::Break(()) = self.step(tr) {
                return true;
            }

            // Hard limit: never spend more time on advancing state than the
            // duration of last frame to prevent stuttering.
            // Of course, this does not safeguard against infinite loops.
            if instant.elapsed().as_secs_f64() >= dt {
                return false;
            }
        }

        true
    }

    pub fn step(&mut self, _tr: &Translator) -> ControlFlow<()> {
        self.master_interactor.step(self.workspace.board_mut())
    }

    pub fn update_appearance_panel(&mut self, ctx: &egui::Context) {
        let Self {
            workspace,
            appearance_panel,
            master_interactor: _,
            dt_accum,
        } = self;

        appearance_panel.update(ctx, workspace.board_mut());
    }

    pub fn handle_input(
        &mut self,
        tr: &Translator,
        ctx: &egui::Context,
        step_rate: Option<f64>,
        scene_to_viewport: egui::emath::TSTransform,
        scene_hovered: bool,
        ui: &mut egui::Ui,
    ) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            let board_master =
                if let MasterInteractor::Autoplacer(interactor) = &self.master_interactor {
                    Some(interactor.board_master().clone())
                } else {
                    None
                };

            if let Workspace::Board(workspace) = &mut self.workspace {
                self.master_interactor.abort(&mut workspace.board);

                if let Some(board_master) = board_master {
                    self.master_interactor = MasterInteractor::Board(board_master);
                }

                *self.workspace.selection_mut() = self.master_interactor.selection().clone();
            }
        }

        let primary_pressed = ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary));
        let primary_down = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
        let primary_released =
            ctx.input(|i| i.pointer.button_released(egui::PointerButton::Primary));
        let delete_pressed = ctx.input(|i| i.key_pressed(egui::Key::Delete));
        let mut maybe_pointer_on_scene: Option<Vector2<i64>> = None;

        if let Some(pointer_viewport_pos) = ctx.input(|i| i.pointer.interact_pos()) {
            let pointer_on_scene_pos = scene_to_viewport.inverse() * pointer_viewport_pos;
            let pointer_on_scene =
                Vector2::new(pointer_on_scene_pos.x as i64, pointer_on_scene_pos.y as i64);
            maybe_pointer_on_scene = Some(pointer_on_scene);

            if primary_pressed && scene_hovered {
                self.master_interactor = MasterInteractor::new(self.workspace.selection().clone());
            }

            if let Workspace::Board(workspace) = &mut self.workspace {
                if primary_down {
                    self.master_interactor.hold(
                        &mut workspace.board,
                        self.appearance_panel.active,
                        pointer_on_scene,
                    );

                    if let Some(select_interactor) =
                        self.master_interactor.select_interactor().as_ref()
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
            let active = self.appearance_panel.active;
            let pointer_for_scene = maybe_pointer_on_scene.unwrap_or_else(|| {
                self.master_interactor
                    .select_interactor()
                    .as_ref()
                    .map(|select_interactor| *select_interactor.origin())
                    .unwrap_or(Vector2::new(0, 0))
            });
            if let Workspace::Board(workspace) = &mut self.workspace {
                self.master_interactor
                    .release(&mut workspace.board, active, pointer_for_scene);
                *self.workspace.selection_mut() = self.master_interactor.selection().clone();
            }
        }

        if delete_pressed {
            if let Workspace::Board(workspace) = &mut self.workspace {
                self.master_interactor.delete(&mut workspace.board);
                *self.workspace.selection_mut() = self.master_interactor.selection().clone();
            }
        }
    }
}
