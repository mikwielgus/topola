// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{controller::Controller, viewport::Viewport};
use topola::{Orientation, Vector2, Workspace};

pub struct DebugOverlay {}

pub struct DebugOverlayOptions {
    pub repulsions: bool,
    pub attractions: bool,
    pub retentions: bool,
    pub bboxes: bool,
    pub navmeshes: bool,
}

impl DebugOverlay {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        viewport: &Viewport,
        workspace: &Controller,
        options: &DebugOverlayOptions,
    ) {
        if options.repulsions {
            self.display_repulsions(ui, viewport, workspace);
        }
        if options.attractions {
            self.display_attractions(ui, viewport, workspace);
        }
        if options.retentions {
            self.display_retentions(ui, viewport, workspace);
        }
        if options.bboxes {
            self.display_bboxes(ctx, ui, viewport, workspace);
        }
        if options.navmeshes {
            self.display_navmeshes(ctx, ui, viewport, workspace);
        }
    }

    fn display_repulsions(&mut self, ui: &egui::Ui, viewport: &Viewport, workspace: &Controller) {
        crate::profile_function!();
        let board = workspace.workspace.board();
        let stroke = egui::Stroke::new(150.0 / viewport.scale_factor(), egui::Color32::YELLOW);

        for selector in &workspace.workspace.selection().components.0 {
            let Some(component_id) = board.component_id(&selector.component) else {
                continue;
            };

            let Some(bbox) = board.layout().component_bbox2(component_id) else {
                continue;
            };

            let origin = Vector2::new((bbox.min.x + bbox.max.x) / 2, (bbox.min.y + bbox.max.y) / 2);

            Self::paint_arrows(
                ui,
                origin,
                board
                    .layout()
                    .locate_component_repulsions(component_id, Orientation::Oblique),
                stroke,
            );
        }

        for selector in &workspace.workspace.selection().pins.0 {
            let Some(pin_id) = board.pin_id(&selector.pin) else {
                continue;
            };

            Self::paint_arrows(
                ui,
                board.layout().pin_centroid(pin_id),
                board
                    .layout()
                    .locate_pin_repulsions(pin_id, Orientation::Oblique),
                stroke,
            );
        }
    }

    fn display_retentions(&mut self, ui: &egui::Ui, viewport: &Viewport, workspace: &Controller) {
        crate::profile_function!();
        let board = workspace.workspace.board();
        let layout = board.layout();
        let stroke = egui::Stroke::new(
            150.0 / viewport.scale_factor(),
            egui::Color32::from_rgb(192, 64, 255),
        );

        for selector in &workspace.workspace.selection().components.0 {
            let Some(component_id) = board.component_id(&selector.component) else {
                continue;
            };

            let Some(bbox) = layout.component_bbox2(component_id) else {
                continue;
            };

            let origin = Vector2::new((bbox.min.x + bbox.max.x) / 2, (bbox.min.y + bbox.max.y) / 2);

            Self::paint_arrows(
                ui,
                origin,
                layout.component_retentions(component_id),
                stroke,
            );
        }

        for selector in &workspace.workspace.selection().pins.0 {
            let Some(pin_id) = board.pin_id(&selector.pin) else {
                continue;
            };

            Self::paint_arrows(
                ui,
                layout.pin_centroid(pin_id),
                layout.pin_retentions(pin_id),
                stroke,
            );
        }
    }

    fn display_attractions(&mut self, ui: &egui::Ui, viewport: &Viewport, workspace: &Controller) {
        crate::profile_function!();
        let board = workspace.workspace.board();
        let layout = board.layout();
        let stroke = egui::Stroke::new(150.0 / viewport.scale_factor(), egui::Color32::BLUE);

        for selector in &workspace.workspace.selection().components.0 {
            let Some(component_id) = board.component_id(&selector.component) else {
                continue;
            };

            let Some(bbox) = layout.component_bbox2(component_id) else {
                continue;
            };

            let origin = Vector2::new((bbox.min.x + bbox.max.x) / 2, (bbox.min.y + bbox.max.y) / 2);

            Self::paint_arrows(
                ui,
                origin,
                layout.component_attractions(component_id),
                stroke,
            );
        }

        for selector in &workspace.workspace.selection().pins.0 {
            let Some(pin_id) = board.pin_id(&selector.pin) else {
                continue;
            };

            Self::paint_arrows(
                ui,
                layout.pin_centroid(pin_id),
                layout.pin_attractions(pin_id),
                stroke,
            );
        }
    }

    fn paint_arrows(
        ui: &egui::Ui,
        origin: Vector2<i64>,
        repulsions: impl IntoIterator<Item = Vector2<i64>>,
        stroke: egui::Stroke,
    ) {
        for repulsion in repulsions {
            if repulsion.x == 0 && repulsion.y == 0 {
                continue;
            }

            ui.painter().arrow(
                egui::pos2(origin.x as f32, origin.y as f32),
                egui::vec2(repulsion.x as f32, repulsion.y as f32),
                stroke,
            );
        }
    }

    fn display_bboxes(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        viewport: &Viewport,
        workspace: &Controller,
    ) {
        crate::profile_function!();
        let board = workspace.workspace.board();
        let layout = board.layout();

        for layer in workspace
            .appearance_panel
            .layers_in_display_order(*layout.layer_count())
        {
            if !workspace.appearance_panel.visible[layer.index()] {
                continue;
            }

            for joint_id in layout.layer_joints(layer) {
                let bbox = layout.joint(joint_id).bbox();
                ui.painter().rect_stroke(
                    egui::Rect {
                        min: egui::pos2(bbox.min.x as f32, bbox.min.y as f32),
                        max: egui::pos2(bbox.max.x as f32, bbox.max.y as f32),
                    },
                    egui::CornerRadius::ZERO,
                    egui::Stroke::new(5.0, egui::Color32::GRAY),
                    egui::StrokeKind::Middle,
                );
            }

            for seg_id in layout.layer_segs(layer) {
                let endpoints = layout.seg(seg_id).endpoints;

                ui.painter().rect_stroke(
                    egui::Rect::from_two_pos(
                        egui::pos2(endpoints[0].x as f32, endpoints[0].y as f32),
                        egui::pos2(endpoints[1].x as f32, endpoints[1].y as f32),
                    ),
                    egui::CornerRadius::ZERO,
                    egui::Stroke::new(5.0, egui::Color32::GRAY),
                    egui::StrokeKind::Middle,
                );
            }

            for via_id in layout.layer_vias(layer) {
                let via = layout.via(via_id);
                let bbox = via.bbox();

                ui.painter().rect_stroke(
                    egui::Rect {
                        min: egui::pos2(bbox.min.x as f32, bbox.min.y as f32),
                        max: egui::pos2(bbox.max.x as f32, bbox.max.y as f32),
                    },
                    egui::CornerRadius::ZERO,
                    egui::Stroke::new(5.0, egui::Color32::GRAY),
                    egui::StrokeKind::Middle,
                );
            }

            for poly_id in layout.layer_polys(layer) {
                let poly = layout.poly(poly_id);
                let bbox = poly.bbox();

                ui.painter().rect_stroke(
                    egui::Rect {
                        min: egui::pos2(bbox.min.x as f32, bbox.min.y as f32),
                        max: egui::pos2(bbox.max.x as f32, bbox.max.y as f32),
                    },
                    egui::CornerRadius::ZERO,
                    egui::Stroke::new(5.0, egui::Color32::GRAY),
                    egui::StrokeKind::Middle,
                );
            }
        }
    }

    fn display_navmeshes(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        viewport: &Viewport,
        workspace: &Controller,
    ) {
        crate::profile_function!();
        let Workspace::Autorouter(autorouter_workspace) = &workspace.workspace else {
            return;
        };
        let autorouter = &autorouter_workspace.autorouter;

        for layer in workspace
            .appearance_panel
            .layers_in_display_order(*workspace.workspace.board().layout().layer_count())
        {
            if workspace.appearance_panel.visible[layer.index()] {
                for navmesh in autorouter
                    .router()
                    .navmesher_board()
                    .navmesher()
                    .layer_navmeshers()[layer.index()]
                .navmeshes()
                {
                    for edge_geom in navmesh
                        .triangulation()
                        .rtreed_dcel()
                        .edges_rtree()
                        .as_ref()
                        .iter()
                    {
                        let (from_vertex, to_vertex) = navmesh
                            .triangulation()
                            .rtreed_dcel()
                            .dcel()
                            .edge_endpoints(edge_geom.data);
                        let from = navmesh
                            .triangulation()
                            .rtreed_dcel()
                            .dcel()
                            .vertex_weight(from_vertex)
                            .position();
                        let to = navmesh
                            .triangulation()
                            .rtreed_dcel()
                            .dcel()
                            .vertex_weight(to_vertex)
                            .position();
                        ui.painter().line_segment(
                            [
                                egui::pos2(*from.x() as f32, *from.y() as f32),
                                egui::pos2(*to.x() as f32, *to.y() as f32),
                            ],
                            egui::Stroke::new(
                                10.0,
                                egui::Color32::WHITE,
                                /*workspace
                                .appearance_panel
                                .colors(ctx)
                                .layers
                                .color(workspace.autorouter.navmesher_board().board().layer_name(layer))
                                .normal,*/
                            ),
                        );
                    }
                }
            }
        }
    }
}
