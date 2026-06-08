// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    controller::Controller,
    debug_overlay::{DebugOverlay, DebugOverlayOptions},
    menu_bar::MenuBar,
    viewport::Viewport,
};
use topola::layout::primitives::{Joint, Poly, Seg, Via};

pub struct Display {}

impl Display {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        viewport: &Viewport,
        workspace: &Controller,
        menu_bar: &MenuBar,
    ) {
        crate::profile_function!();

        self.display_layout(ctx, ui, viewport, workspace);
        self.display_ratsnest(ctx, ui, viewport, workspace);

        let mut debug_overlay = DebugOverlay::new();
        debug_overlay.update(
            ctx,
            ui,
            viewport,
            workspace,
            &DebugOverlayOptions {
                repulsions: menu_bar.show_repulsions,
                attractions: menu_bar.show_attractions,
                retentions: menu_bar.show_retentions,
                bboxes: menu_bar.show_bboxes,
                navmeshes: menu_bar.show_navmeshes,
            },
        );
    }

    fn display_layout(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        //menu_bar: &MenuBar,
        viewport: &Viewport,
        workspace: &Controller,
    ) {
        crate::profile_function!();
        let board = workspace.workspace.board();
        let layout = board.layout();

        // Start from the bottom layer so that top layers are drawn on top.
        // The active layer is drawn last so it stays visible above the rest.
        for layer in workspace
            .appearance_panel
            .layers_in_display_order(*layout.layer_count())
        {
            if !workspace.appearance_panel.visible[layer.index()] {
                continue;
            }

            for joint_id in layout.layer_joints(layer) {
                let joint = layout.joint(joint_id);
                let pin_selected =
                    board.pins_contain_joint(&workspace.workspace.selection().pins, joint_id);
                let net_selected =
                    board.nets_contain_joint(&workspace.workspace.selection().nets, joint_id);
                let component_selected = board.components_contain_joint(
                    &workspace.workspace.selection().components,
                    joint_id,
                );
                self.paint_joint(
                    ctx,
                    ui,
                    viewport,
                    joint,
                    workspace.appearance_panel.layer_color(
                        ctx,
                        board.layer_desc(joint.spec.layer),
                        pin_selected,
                        (joint.spec.pin.is_none() && net_selected) || component_selected,
                    ),
                );
            }

            for seg_id in layout.layer_segs(layer) {
                let seg = layout.seg(seg_id);
                let pin_selected =
                    board.pins_contain_seg(&workspace.workspace.selection().pins, seg_id);
                let net_selected =
                    board.nets_contain_seg(&workspace.workspace.selection().nets, seg_id);
                let component_selected = board
                    .components_contain_seg(&workspace.workspace.selection().components, seg_id);
                self.paint_seg(
                    ctx,
                    ui,
                    viewport,
                    seg,
                    workspace.appearance_panel.layer_color(
                        ctx,
                        board.layer_desc(seg.layer),
                        pin_selected,
                        (seg.spec.pin.is_none() && net_selected) || component_selected,
                    ),
                );
            }

            for via_id in layout.layer_vias(layer) {
                let via = layout.via(via_id);
                let pin_selected =
                    board.pins_contain_via(&workspace.workspace.selection().pins, via_id);
                let net_selected =
                    board.nets_contain_via(&workspace.workspace.selection().nets, via_id);
                let component_selected = board
                    .components_contain_via(&workspace.workspace.selection().components, via_id);
                self.paint_via(
                    ctx,
                    ui,
                    viewport,
                    via,
                    workspace.appearance_panel.layer_color(
                        ctx,
                        board.layer_desc(layer),
                        pin_selected,
                        (via.spec.pin.is_none() && net_selected) || component_selected,
                    ),
                );
            }

            for poly_id in layout.layer_polys(layer) {
                let poly = layout.poly(poly_id);
                let pin_selected =
                    board.pins_contain_poly(&workspace.workspace.selection().pins, poly_id);
                let net_selected =
                    board.nets_contain_poly(&workspace.workspace.selection().nets, poly_id);
                let component_selected = board
                    .components_contain_poly(&workspace.workspace.selection().components, poly_id);
                self.paint_poly(
                    ctx,
                    ui,
                    viewport,
                    poly,
                    workspace.appearance_panel.layer_color(
                        ctx,
                        board.layer_desc(poly.spec.layer),
                        pin_selected,
                        (poly.spec.pin.is_none() && net_selected) || component_selected,
                    ),
                );
            }
        }

        ui.painter().line(
            layout
                .boundary()
                .iter()
                .map(|p| egui::Pos2 {
                    x: p.x as f32,
                    y: p.y as f32,
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
            egui::pos2(joint.spec.position.x as f32, joint.spec.position.y as f32),
            joint.spec.radius as f32,
            color,
        );
    }

    fn paint_seg(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        viewport: &Viewport,
        seg: &Seg,
        color: egui::Color32,
    ) {
        ui.painter().line_segment(
            [
                egui::pos2(seg.endpoints[0].x as f32, seg.endpoints[0].y as f32),
                egui::pos2(seg.endpoints[1].x as f32, seg.endpoints[1].y as f32),
            ],
            egui::Stroke::new(seg.spec.half_width as f32 * 2.0, color),
        );
    }

    fn paint_via(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        viewport: &Viewport,
        via: &Via,
        color: egui::Color32,
    ) {
        ui.painter().circle_filled(
            egui::pos2(via.position.x as f32, via.position.y as f32),
            via.spec.radius as f32,
            color,
        );
    }

    fn paint_poly(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        viewport: &Viewport,
        poly: &Poly,
        color: egui::Color32,
    ) {
        let points: Vec<egui::Pos2> = poly
            .spec
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

    fn display_ratsnest(
        &mut self,
        _ctx: &egui::Context,
        ui: &egui::Ui,
        _viewport: &Viewport,
        workspace: &Controller,
    ) {
        crate::profile_function!();

        for ratline in workspace.workspace.ratsnest().ratlines() {
            let layers = *ratline.endpoint_layers();
            let endpoints = *ratline.endpoints();

            if !workspace.appearance_panel.visible[layers[0].index()]
                || !workspace.appearance_panel.visible[layers[1].index()]
            {
                continue;
            }

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
