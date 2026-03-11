// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{viewport::Viewport, workspace::Workspace};
use topola::{Joint, Polygon, Segment, SegmentId};

pub struct Displayer {}

impl Displayer {
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
            egui::pos2(joint.position[0] as f32, joint.position[1] as f32),
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
        endpoints: [[i64; 2]; 2],
        color: egui::Color32,
    ) {
        ui.painter().line_segment(
            [
                egui::pos2(endpoints[0][0] as f32, endpoints[0][1] as f32),
                egui::pos2(endpoints[1][0] as f32, endpoints[1][1] as f32),
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
            .map(|v| egui::pos2(v[0] as f32, v[1] as f32))
            .collect();

        ui.painter().add(egui::Shape::convex_polygon(
            points,
            color,
            egui::Stroke::new(5.0 / viewport.scale_factor(), color),
        ));
    }
}
