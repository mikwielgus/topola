// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{viewport::Viewport, workspace::Workspace};
use topola::{Joint, Polygon, Segment, Vector2};

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
        self.display_ratsnest(ctx, ui, viewport, workspace);
    }

    fn display_layout(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        //menu_bar: &MenuBar,
        viewport: &Viewport,
        workspace: &Workspace,
    ) {
        let board = workspace.autorouter.navmesher_board().board();
        let layout = board.layout();

        // Start from the bottom layer so that top layers are drawn on top.
        for layer in (0..*layout.layer_count()).rev() {
            if !workspace.appearance_panel.visible[layer] {
                continue;
            }

            for joint_id in layout.layer_joints(layer) {
                let joint = layout.joint(joint_id);
                self.paint_joint(
                    ctx,
                    ui,
                    viewport,
                    joint,
                    workspace.appearance_panel.layer_color(
                        ctx,
                        board.layer_name(joint.layer),
                        board.pin_selection_contains_joint(&workspace.pin_selection, joint_id),
                    ),
                );
            }

            for segment_id in layout.layer_segments(layer) {
                let segment = layout.segment(segment_id);
                self.paint_segment(
                    ctx,
                    ui,
                    viewport,
                    segment,
                    layout.segment_endpoints(segment_id),
                    workspace.appearance_panel.layer_color(
                        ctx,
                        board.layer_name(segment.layer),
                        board.pin_selection_contains_segment(&workspace.pin_selection, segment_id),
                    ),
                );
            }

            // TODO: Vias.

            for polygon_id in layout.layer_polygons(layer) {
                let polygon = layout.polygon(polygon_id);
                self.paint_polygon(
                    ctx,
                    ui,
                    viewport,
                    polygon,
                    workspace.appearance_panel.layer_color(
                        ctx,
                        board.layer_name(polygon.layer),
                        board.pin_selection_contains_polygon(&workspace.pin_selection, polygon_id),
                    ),
                );
            }
        }

        ui.painter().line(
            layout
                .boundary()
                .iter()
                .map(|p| egui::Pos2 {
                    x: p[0] as f32,
                    y: p[1] as f32,
                })
                .collect::<Vec<_>>(),
            egui::Stroke::new(5.0 / viewport.scale_factor(), egui::Color32::WHITE),
        );
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
        let board = workspace.autorouter.navmesher_board().board();
        let layout = board.layout();

        for layer in (0..*layout.layer_count()).rev() {
            if !workspace.appearance_panel.visible[layer] {
                continue;
            }

            for joint_id in layout.layer_joints(layer) {
                let joint = layout.joint(joint_id);
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

            for segment_id in layout.layer_segments(layer) {
                let endpoints = layout.segment_endpoints(segment_id);

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

            // TODO: vias.

            for polygon_id in layout.layer_polygons(layer) {
                let polygon = layout.polygon(polygon_id);
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
        for layer in 0..*workspace
            .autorouter
            .navmesher_board()
            .board()
            .layout()
            .layer_count()
        {
            if workspace.appearance_panel.visible[layer] {
                for navmesh in workspace
                    .autorouter
                    .navmesher_board()
                    .navmesher()
                    .layer_navmeshers()[layer]
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

    fn display_ratsnest(
        &mut self,
        _ctx: &egui::Context,
        ui: &egui::Ui,
        _viewport: &Viewport,
        workspace: &Workspace,
    ) {
        for ratline in workspace.autorouter.ratsnest().ratlines() {
            let layers = *ratline.endpoint_layers();
            let endpoints = *ratline.endpoints();

            if !workspace.appearance_panel.visible[layers[0]]
                || !workspace.appearance_panel.visible[layers[1]]
            {
                continue;
            }

            //let stroke_width = 2.0 / viewport.scale_factor().max(1e-6);

            ui.painter().line_segment(
                [
                    egui::pos2(endpoints[0].x as f32, endpoints[0].y as f32),
                    egui::pos2(endpoints[1].x as f32, endpoints[1].y as f32),
                ],
                egui::Stroke::new(10.0, egui::Color32::BLUE),
            );
        }
    }
}
