// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{viewport::Viewport, workspace::Workspace};
use topola::{Joint, Polygon, Segment, SegmentId, Vector2};

pub struct Display {}

impl Display {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        //menu_bar: &MenuBar,
        viewport: &Viewport,
        workspace: &Workspace,
    ) {
        self.display_layout(ctx, ui, /*menu_bar,*/ viewport, workspace);
        self.display_bboxes(ctx, ui, viewport, workspace);
        self.display_navmeshes(ctx, ui, viewport, workspace);
    }

    pub fn display_layout(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        //menu_bar: &MenuBar,
        viewport: &Viewport,
        workspace: &Workspace,
    ) {
        ui.painter().line(
            workspace
                .navmesher_board
                .board()
                .layout()
                .boundary()
                .iter()
                .map(|p| egui::Pos2 {
                    x: p[0] as f32,
                    y: p[1] as f32,
                })
                .collect::<Vec<_>>(),
            egui::Stroke::new(5.0 / viewport.scale_factor(), egui::Color32::WHITE),
        );

        for (_, joint) in workspace
            .navmesher_board
            .board()
            .layout()
            .joints()
            .collection()
        {
            if workspace.appearance_panel.visible[joint.layer] {
                self.paint_joint(
                    ctx,
                    ui,
                    viewport,
                    joint,
                    workspace
                        .appearance_panel
                        .colors(ctx)
                        .layers
                        .color(workspace.navmesher_board.board().layer_name(joint.layer))
                        .normal,
                );
            }
        }

        for (i, segment) in workspace
            .navmesher_board
            .board()
            .layout()
            .segments()
            .collection()
        {
            if workspace.appearance_panel.visible[segment.layer] {
                self.paint_segment(
                    ctx,
                    ui,
                    viewport,
                    segment,
                    workspace
                        .navmesher_board
                        .board()
                        .layout()
                        .segment_endpoints(SegmentId::new(i)),
                    workspace
                        .appearance_panel
                        .colors(ctx)
                        .layers
                        .color(workspace.navmesher_board.board().layer_name(segment.layer))
                        .normal,
                );
            }
        }

        for (_, polygon) in workspace
            .navmesher_board
            .board()
            .layout()
            .polygons()
            .collection()
        {
            if workspace.appearance_panel.visible[polygon.layer] {
                self.paint_polygon(
                    ctx,
                    ui,
                    viewport,
                    polygon,
                    workspace
                        .appearance_panel
                        .colors(ctx)
                        .layers
                        .color(workspace.navmesher_board.board().layer_name(polygon.layer))
                        .normal,
                );
            }
        }
    }

    fn paint_joint(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        viewport: &Viewport,
        joint: &Joint,
        color: egui::Color32,
    ) {
        ui.painter().circle_filled(
            egui::pos2(joint.position.x as f32, joint.position.y as f32),
            joint.radius as f32,
            color,
        );
    }

    fn paint_segment(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        viewport: &Viewport,
        segment: &Segment,
        endpoints: [Vector2<i64>; 2],
        color: egui::Color32,
    ) {
        ui.painter().line_segment(
            [
                egui::pos2(endpoints[0].x as f32, endpoints[0].y as f32),
                egui::pos2(endpoints[1].x as f32, endpoints[1].y as f32),
            ],
            egui::Stroke::new(segment.half_width as f32 * 2.0, color),
        );
    }

    fn paint_polygon(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        viewport: &Viewport,
        polygon: &Polygon,
        color: egui::Color32,
    ) {
        let points: Vec<egui::Pos2> = polygon
            .vertices
            .iter()
            .map(|v| egui::pos2(v.x as f32, v.y as f32))
            .collect();

        ui.painter().add(egui::Shape::convex_polygon(
            points,
            color,
            egui::Stroke::new(5.0 / viewport.scale_factor(), color),
        ));
    }

    fn display_bboxes(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        viewport: &Viewport,
        workspace: &Workspace,
    ) {
        for (_, joint) in workspace
            .navmesher_board
            .board()
            .layout()
            .joints()
            .collection()
        {
            if workspace.appearance_panel.visible[joint.layer] {
                ui.painter().rect_stroke(
                    egui::Rect {
                        min: egui::pos2(
                            joint.bbox().lower()[0] as f32,
                            joint.bbox().lower()[1] as f32,
                        ),
                        max: egui::pos2(
                            joint.bbox().upper()[0] as f32,
                            joint.bbox().upper()[1] as f32,
                        ),
                    },
                    egui::CornerRadius::ZERO,
                    egui::Stroke::new(5.0, egui::Color32::GRAY),
                    egui::StrokeKind::Middle,
                );
            }
        }

        for (i, segment) in workspace
            .navmesher_board
            .board()
            .layout()
            .segments()
            .collection()
        {
            if workspace.appearance_panel.visible[segment.layer] {
                let endpoints = workspace
                    .navmesher_board
                    .board()
                    .layout()
                    .segment_endpoints(SegmentId::new(i));

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
        }

        // TODO: vias.

        for (i, polygon) in workspace
            .navmesher_board
            .board()
            .layout()
            .polygons()
            .collection()
        {
            if workspace.appearance_panel.visible[polygon.layer] {
                let bbox = polygon.bbox();

                ui.painter().rect_stroke(
                    egui::Rect {
                        min: egui::pos2(bbox.lower()[0] as f32, bbox.lower()[1] as f32),
                        max: egui::pos2(bbox.upper()[0] as f32, bbox.upper()[1] as f32),
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
        workspace: &Workspace,
    ) {
        for layer in 0..*workspace.navmesher_board.board().layout().layer_count() {
            if workspace.appearance_panel.visible[layer] {
                for navmesh in workspace.navmesher_board.navmesher().layers()[layer].navmeshes() {
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
                                .color(workspace.navmesher_board.board().layer_name(layer))
                                .normal,*/
                            ),
                        );
                    }
                }
            }
        }
    }
}
