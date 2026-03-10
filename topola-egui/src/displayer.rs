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
                .board
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

        for (_, joint) in workspace.board.layout().joints().collection() {
            self.paint_joint(ctx, ui, viewport, joint);
        }

        for (i, segment) in workspace.board.layout().segments().collection() {
            self.paint_segment(
                ctx,
                ui,
                viewport,
                segment,
                workspace
                    .board
                    .layout()
                    .segment_endpoints(SegmentId::new(i)),
            );
        }

        for (_, polygon) in workspace.board.layout().polygons().collection() {
            self.paint_polygon(ctx, ui, viewport, polygon);
        }
    }

    fn paint_joint(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        viewport: &Viewport,
        joint: &Joint,
    ) {
        ui.painter().circle_filled(
            egui::pos2(joint.position[0] as f32, joint.position[1] as f32),
            joint.radius as f32,
            egui::Color32::RED,
        );
    }

    fn paint_segment(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        viewport: &Viewport,
        segment: &Segment,
        endpoints: [[i64; 2]; 2],
    ) {
        ui.painter().line_segment(
            [
                egui::pos2(endpoints[0][0] as f32, endpoints[0][1] as f32),
                egui::pos2(endpoints[1][0] as f32, endpoints[1][1] as f32),
            ],
            egui::Stroke::new(segment.half_width as f32 * 2.0, egui::Color32::RED),
        );
    }

    fn paint_polygon(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        viewport: &Viewport,
        polygon: &Polygon,
    ) {
        let points: Vec<egui::Pos2> = polygon
            .vertices
            .iter()
            .map(|v| egui::pos2(v[0] as f32, v[1] as f32))
            .collect();

        ui.painter().add(egui::Shape::convex_polygon(
            points,
            egui::Color32::RED,
            egui::Stroke::new(5.0 / viewport.scale_factor(), egui::Color32::RED),
        ));
    }
}
