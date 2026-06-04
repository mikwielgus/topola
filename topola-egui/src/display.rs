// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{viewport::Viewport, workspace::GuiWorkspace};
use topola::{
    Orientation, Vector2, Workspace,
    layout::primitives::{Joint, Polygon, Segment, Via},
};

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
        workspace: &GuiWorkspace,
    ) {
        self.display_layout(ctx, ui, /*menu_bar,*/ viewport, workspace);
        self.display_repulsions(ui, viewport, workspace);
        self.display_attractions(ui, viewport, workspace);
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
        workspace: &GuiWorkspace,
    ) {
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

            for segment_id in layout.layer_segments(layer) {
                let segment = layout.segment(segment_id);
                let pin_selected =
                    board.pins_contain_segment(&workspace.workspace.selection().pins, segment_id);
                let net_selected =
                    board.nets_contain_segment(&workspace.workspace.selection().nets, segment_id);
                let component_selected = board.components_contain_segment(
                    &workspace.workspace.selection().components,
                    segment_id,
                );
                self.paint_segment(
                    ctx,
                    ui,
                    viewport,
                    segment,
                    workspace.appearance_panel.layer_color(
                        ctx,
                        board.layer_desc(segment.layer),
                        pin_selected,
                        (segment.spec.pin.is_none() && net_selected) || component_selected,
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

            for polygon_id in layout.layer_polygons(layer) {
                let polygon = layout.polygon(polygon_id);
                let pin_selected =
                    board.pins_contain_polygon(&workspace.workspace.selection().pins, polygon_id);
                let net_selected =
                    board.nets_contain_polygon(&workspace.workspace.selection().nets, polygon_id);
                let component_selected = board.components_contain_polygon(
                    &workspace.workspace.selection().components,
                    polygon_id,
                );
                self.paint_polygon(
                    ctx,
                    ui,
                    viewport,
                    polygon,
                    workspace.appearance_panel.layer_color(
                        ctx,
                        board.layer_desc(polygon.layer),
                        pin_selected,
                        (polygon.pin.is_none() && net_selected) || component_selected,
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

    fn display_repulsions(&mut self, ui: &egui::Ui, viewport: &Viewport, workspace: &GuiWorkspace) {
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

    fn display_attractions(
        &mut self,
        ui: &egui::Ui,
        viewport: &Viewport,
        workspace: &GuiWorkspace,
    ) {
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

    fn paint_segment(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        viewport: &Viewport,
        segment: &Segment,
        color: egui::Color32,
    ) {
        ui.painter().line_segment(
            [
                egui::pos2(segment.endpoints[0].x as f32, segment.endpoints[0].y as f32),
                egui::pos2(segment.endpoints[1].x as f32, segment.endpoints[1].y as f32),
            ],
            egui::Stroke::new(segment.spec.half_width as f32 * 2.0, color),
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
        workspace: &GuiWorkspace,
    ) {
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

            for segment_id in layout.layer_segments(layer) {
                let endpoints = layout.segment(segment_id).endpoints;

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

            for polygon_id in layout.layer_polygons(layer) {
                let polygon = layout.polygon(polygon_id);
                let bbox = polygon.bbox();

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
        workspace: &GuiWorkspace,
    ) {
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

    fn display_ratsnest(
        &mut self,
        _ctx: &egui::Context,
        ui: &egui::Ui,
        _viewport: &Viewport,
        workspace: &GuiWorkspace,
    ) {
        let Workspace::Autorouter(autorouter_workspace) = &workspace.workspace else {
            return;
        };
        let autorouter = &autorouter_workspace.autorouter;

        for ratline in autorouter.ratsnest().ratlines() {
            let layers = *ratline.endpoint_layers();
            let endpoints = *ratline.endpoints();

            if !workspace.appearance_panel.visible[layers[0].index()]
                || !workspace.appearance_panel.visible[layers[1].index()]
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
