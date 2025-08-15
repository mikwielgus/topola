// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use petgraph::{
    data::DataMap,
    visit::{EdgeRef, IntoEdgeReferences},
};
use rstar::AABB;
use topola::{
    autorouter::invoker::GetDebugOverlayData,
    board::AccessMesadata,
    drawing::{
        bend::BendIndex,
        dot::DotIndex,
        graph::{MakePrimitive, PrimitiveIndex},
        head::GetFace,
        primitive::MakePrimitiveShape,
    },
    geometry::{shape::AccessShape, GenericNode},
    graph::MakeRef,
    interactor::{activity::ActivityStepper, interaction::InteractionStepper},
    layout::poly::MakePolygon,
    math::{Circle, RotationSense},
    router::{
        navmesh::{BinavnodeNodeIndex, NavnodeIndex},
        ng::pie,
        prenavmesh::PrenavmeshConstraint,
    },
};

use crate::{config::Config, menu_bar::MenuBar, painter::Painter, workspace::Workspace};

pub struct Displayer<'a> {
    config: &'a Config,
    painter: Painter<'a>,
    workspace: &'a mut Workspace,
}

impl<'a> Displayer<'a> {
    pub fn new(config: &'a Config, painter: Painter<'a>, workspace: &'a mut Workspace) -> Self {
        Self {
            config,
            painter,
            workspace,
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, menu_bar: &MenuBar) {
        self.display_layout(ctx);

        if menu_bar.show_ratsnest {
            self.display_ratsnest();
        }

        if menu_bar.show_navmesh || menu_bar.show_guide_circles {
            self.display_navmesh_or_guides(menu_bar);
        }

        if menu_bar.show_triangulation {
            self.display_triangulation();
        }

        if menu_bar.show_triangulation_constraints {
            self.display_triangulation_constraints();
        }

        if menu_bar.show_topo_navmesh {
            self.display_topo_navmesh();
        }

        if menu_bar.show_bboxes {
            self.display_bboxes();
        }

        self.display_activity(menu_bar);
    }

    fn display_layout(&mut self, ctx: &egui::Context) {
        let board = self.workspace.interactor.invoker().autorouter().board();
        let active_polygons = self
            .workspace
            .interactor
            .maybe_activity()
            .as_ref()
            .map(|i| i.active_polygons())
            .unwrap_or_default();

        for i in (0..self.workspace.appearance_panel.visible.len()).rev() {
            if self.workspace.appearance_panel.visible[i] {
                for primitive in board.layout().drawing().layer_primitive_nodes(i) {
                    let shape = primitive.primitive(board.layout().drawing()).shape();

                    let color = if self
                        .workspace
                        .overlay
                        .selection()
                        .contains_node(board, GenericNode::Primitive(primitive))
                    {
                        self.config
                            .colors(ctx)
                            .layers
                            .color(board.layout().rules().layer_layername(i))
                            .highlighted
                    } else if let Some(activity) = &mut self.workspace.interactor.maybe_activity() {
                        if activity.obstacles().contains(&primitive) {
                            self.config
                                .colors(ctx)
                                .layers
                                .color(board.layout().rules().layer_layername(i))
                                .highlighted
                        } else {
                            self.config
                                .colors(ctx)
                                .layers
                                .color(board.layout().rules().layer_layername(i))
                                .normal
                        }
                    } else {
                        self.config
                            .colors(ctx)
                            .layers
                            .color(board.layout().rules().layer_layername(i))
                            .normal
                    };

                    self.painter.paint_primitive(&shape, color);
                }

                for poly in board.layout().layer_poly_nodes(i) {
                    let color = if self
                        .workspace
                        .overlay
                        .selection()
                        .contains_node(board, GenericNode::Compound(poly.into()))
                        || active_polygons.iter().find(|&&i| i == poly).is_some()
                    {
                        self.config
                            .colors(ctx)
                            .layers
                            .color(board.layout().rules().layer_layername(i))
                            .highlighted
                    } else {
                        self.config
                            .colors(ctx)
                            .layers
                            .color(board.layout().rules().layer_layername(i))
                            .normal
                    };

                    self.painter
                        .paint_polygon(&poly.ref_(board.layout()).shape(), color)
                }
            }
        }
    }

    fn display_ratsnest(&mut self) {
        let graph = self.workspace.overlay.ratsnest().graph();
        for edge in graph.edge_references() {
            let from = graph.node_weight(edge.source()).unwrap().pos;
            let to = graph.node_weight(edge.target()).unwrap().pos;

            self.painter.paint_line_segment(
                from,
                to,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 90, 200)),
            );
        }
    }

    fn display_navmesh_or_guides(&mut self, menu_bar: &MenuBar) {
        let board = self.workspace.interactor.invoker().autorouter().board();

        if let Some(activity) = self.workspace.interactor.maybe_activity() {
            if let Some(thetastar) = activity.maybe_thetastar() {
                let navmesh = thetastar.graph();

                for edge in navmesh.edge_references() {
                    let mut from =
                        PrimitiveIndex::from(navmesh.node_weight(edge.source()).unwrap().node)
                            .primitive(board.layout().drawing())
                            .shape()
                            .center();
                    let mut to =
                        PrimitiveIndex::from(navmesh.node_weight(edge.target()).unwrap().node)
                            .primitive(board.layout().drawing())
                            .shape()
                            .center();

                    if let Some(from_sense) =
                        navmesh.node_weight(edge.source()).unwrap().maybe_sense
                    {
                        from += match from_sense {
                            RotationSense::Counterclockwise => [0.0, 150.0].into(),
                            RotationSense::Clockwise => [-0.0, -150.0].into(),
                        };
                    }

                    if let Some(to_sense) = navmesh.node_weight(edge.target()).unwrap().maybe_sense
                    {
                        to += match to_sense {
                            RotationSense::Counterclockwise => [0.0, 150.0].into(),
                            RotationSense::Clockwise => [-0.0, -150.0].into(),
                        }
                    }

                    let stroke = 'blk: {
                        if let Some(navcord) = activity.maybe_navcord() {
                            if let (Some(source_pos), Some(target_pos)) = (
                                navcord.path.iter().position(|node| *node == edge.source()),
                                navcord.path.iter().position(|node| *node == edge.target()),
                            ) {
                                if target_pos == source_pos + 1 || source_pos == target_pos + 1 {
                                    break 'blk egui::Stroke::new(
                                        5.0,
                                        egui::Color32::from_rgb(250, 250, 0),
                                    );
                                }
                            }
                        }

                        egui::Stroke::new(1.0, egui::Color32::from_rgb(125, 125, 125))
                    };

                    if menu_bar.show_navmesh {
                        self.painter.paint_line_segment(from, to, stroke);
                    }

                    if let Some(text) = activity.navedge_debug_text((edge.source(), edge.target()))
                    {
                        self.painter.paint_text(
                            (from + to) / 2.0,
                            egui::Align2::LEFT_BOTTOM,
                            text,
                            egui::Color32::from_rgb(255, 255, 255),
                        );
                    }
                }

                for index in navmesh.graph().node_indices() {
                    let navnode = NavnodeIndex(index);
                    let primitive =
                        PrimitiveIndex::from(navmesh.node_weight(navnode).unwrap().node);
                    let mut pos = primitive
                        .primitive(board.layout().drawing())
                        .shape()
                        .center();

                    if menu_bar.show_guide_circles {
                        if let Some(navcord) = activity.maybe_navcord() {
                            if let Ok(dot) = DotIndex::try_from(primitive) {
                                let drawing = board.layout().drawing();

                                self.painter.paint_hollow_circle(
                                    drawing.dot_circle(
                                        dot,
                                        navcord.width,
                                        drawing.conditions(navcord.head.face().into()).as_ref(),
                                    ),
                                    1.0,
                                    egui::epaint::Color32::WHITE,
                                );
                            } else if let Ok(bend) = BendIndex::try_from(primitive) {
                                let drawing = board.layout().drawing();

                                self.painter.paint_hollow_circle(
                                    drawing.bend_circle(
                                        bend,
                                        navcord.width,
                                        drawing.conditions(navcord.head.face().into()).as_ref(),
                                    ),
                                    1.0,
                                    egui::epaint::Color32::WHITE,
                                );
                            }
                        }
                    }

                    pos += match navmesh.node_weight(navnode).unwrap().maybe_sense {
                        Some(RotationSense::Counterclockwise) => [0.0, 150.0].into(),
                        Some(RotationSense::Clockwise) => [-0.0, -150.0].into(),
                        None => [0.0, 0.0].into(),
                    };

                    if menu_bar.show_pathfinding_scores {
                        let score_text = thetastar
                            .scores()
                            .get(&navnode)
                            .map_or_else(String::new, |s| format!("g={:.2}", s));
                        let estimate_score_text = thetastar
                            .cost_to_goal_estimate_scores()
                            .get(&navnode)
                            .map_or_else(String::new, |s| format!("(f={:.2})", s));
                        let debug_text = activity.navnode_debug_text(navnode).unwrap_or("");

                        self.painter.paint_text(
                            pos,
                            egui::Align2::LEFT_BOTTOM,
                            &format!("{} {} {}", score_text, estimate_score_text, debug_text),
                            egui::Color32::from_rgb(255, 255, 255),
                        );
                    }
                }
            }
        }
    }

    fn display_triangulation(&mut self) {
        let board = self.workspace.interactor.invoker().autorouter().board();

        if let Some(activity) = self.workspace.interactor.maybe_activity() {
            if let Some(thetastar) = activity.maybe_thetastar() {
                let navmesh = thetastar.graph();

                for edge in navmesh.prenavmesh().triangulation().edge_references() {
                    let from = PrimitiveIndex::from(BinavnodeNodeIndex::from(edge.source()))
                        .primitive(board.layout().drawing())
                        .shape()
                        .center();
                    let to = PrimitiveIndex::from(BinavnodeNodeIndex::from(edge.target()))
                        .primitive(board.layout().drawing())
                        .shape()
                        .center();

                    self.painter.paint_line_segment(
                        from,
                        to,
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 255, 255)),
                    );
                }
            }
        }
    }

    fn display_triangulation_constraints(&mut self) {
        if let Some(activity) = self.workspace.interactor.maybe_activity() {
            if let Some(thetastar) = activity.maybe_thetastar() {
                let navmesh = thetastar.graph();

                for PrenavmeshConstraint(from_weight, to_weight) in
                    navmesh.prenavmesh().constraints().iter()
                {
                    let from = from_weight.pos + [100.0, 100.0].into();
                    let to = to_weight.pos + [100.0, 100.0].into();

                    self.painter.paint_line_segment(
                        from,
                        to,
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 255, 0)),
                    )
                }
            }
        }
    }

    fn display_topo_navmesh(&mut self) {
        let board = self.workspace.interactor.invoker().autorouter().board();

        if let Some(navmesh) = self
            .workspace
            .interactor
            .maybe_activity()
            .as_ref()
            .and_then(|i| i.maybe_topo_navmesh())
            .or_else(|| {
                self.workspace
                    .overlay
                    .planar_incr_navmesh()
                    .as_ref()
                    .map(|navmesh| navmesh.as_ref())
            })
        {
            // calculate dual node position approximations
            use std::collections::BTreeMap;
            use topola::geometry::shape::AccessShape;
            use topola::router::ng::pie::NavmeshIndex;
            let mut map = BTreeMap::new();
            let resolve_primal = |p: &topola::drawing::dot::FixedDotIndex| {
                (*p).primitive(board.layout().drawing()).shape().center()
            };

            for (nidx, node) in &*navmesh.nodes {
                if let NavmeshIndex::Dual(didx) = nidx {
                    map.insert(didx, geo::point! { x: node.pos.x, y: node.pos.y });
                }
            }
            for (eidx, edge) in &*navmesh.edges {
                // TODO: display edge contents, too
                let (a, b) = (*eidx).into();
                let mut got_primal = false;
                let a_pos = match a {
                    NavmeshIndex::Primal(p) => {
                        got_primal = true;
                        resolve_primal(&p)
                    }
                    NavmeshIndex::Dual(d) => match map.get(&d) {
                        None => continue,
                        Some(&x) => x,
                    },
                };
                let b_pos = match b {
                    NavmeshIndex::Primal(p) => {
                        got_primal = true;
                        resolve_primal(&p)
                    }
                    NavmeshIndex::Dual(d) => match map.get(&d) {
                        None => continue,
                        Some(&x) => x,
                    },
                };
                let lhs_pos = edge.0.lhs.map(|i| resolve_primal(&i));
                let rhs_pos = edge.0.rhs.map(|i| resolve_primal(&i));
                use egui::Color32;
                let make_stroke = |len: usize| {
                    if len == 0 {
                        egui::Stroke::new(
                            0.5,
                            if got_primal {
                                Color32::from_rgb(255, 175, 0)
                            } else {
                                Color32::from_rgb(159, 255, 33)
                            },
                        )
                    } else {
                        egui::Stroke::new(1.0 + (len as f32).atan(), Color32::from_rgb(250, 250, 0))
                    }
                };
                if let (Some(lhs), Some(rhs)) = (lhs_pos, rhs_pos) {
                    let edge_lens = navmesh.edge_paths[edge.1]
                        .split(|x| x == &pie::RelaxedPath::Weak(()))
                        .collect::<Vec<_>>();
                    assert_eq!(edge_lens.len(), 3);
                    let middle = (a_pos + b_pos) / 2.0;
                    let mut offset_lhs = lhs - middle;
                    offset_lhs /= offset_lhs.dot(offset_lhs).sqrt() / 50.0;
                    let mut offset_rhs = rhs - middle;
                    offset_rhs /= offset_rhs.dot(offset_rhs).sqrt() / 50.0;
                    self.painter.paint_line_segment(
                        a_pos + offset_lhs,
                        b_pos + offset_lhs,
                        make_stroke(edge_lens[0].len()),
                    );
                    self.painter
                        .paint_line_segment(a_pos, b_pos, make_stroke(edge_lens[1].len()));
                    self.painter.paint_line_segment(
                        a_pos + offset_rhs,
                        b_pos + offset_rhs,
                        make_stroke(edge_lens[2].len()),
                    );
                } else {
                    let edge_len = navmesh.edge_paths[edge.1]
                        .iter()
                        .filter(|i| matches!(i, pie::RelaxedPath::Normal(_)))
                        .count();
                    self.painter
                        .paint_line_segment(a_pos, b_pos, make_stroke(edge_len));
                }
            }
        }
    }

    fn display_bboxes(&mut self) {
        let board = self.workspace.interactor.invoker().autorouter().board();

        let root_bbox3d = board.layout().drawing().rtree().root().envelope();
        let root_bbox = AABB::<[f64; 2]>::from_corners(
            [root_bbox3d.lower()[0], root_bbox3d.lower()[1]],
            [root_bbox3d.upper()[0], root_bbox3d.upper()[1]],
        );

        self.painter.paint_bbox(root_bbox);
    }

    fn display_activity(&mut self, menu_bar: &MenuBar) {
        let board = self.workspace.interactor.invoker().autorouter().board();

        if let Some(activity) = self.workspace.interactor.maybe_activity() {
            for ghost in activity.ghosts() {
                self.painter
                    .paint_primitive(ghost, egui::Color32::from_rgb(75, 75, 150));
            }

            if let ActivityStepper::Interaction(InteractionStepper::RoutePlan(rp)) =
                activity.activity()
            {
                self.painter
                    .paint_polyline(&rp.lines, egui::Color32::from_rgb(245, 182, 66));
            }

            for linestring in activity.polygonal_blockers() {
                self.painter
                    .paint_polyline(linestring, egui::Color32::from_rgb(115, 0, 255));
            }

            if let Some(ref navmesh) = activity.maybe_thetastar().map(|astar| astar.graph()) {
                if menu_bar.show_origin_destination {
                    let (origin, destination) = (navmesh.origin(), navmesh.destination());
                    self.painter.paint_solid_circle(
                        Circle {
                            pos: board.layout().drawing().primitive(origin).shape().center(),
                            r: 150.0,
                        },
                        egui::Color32::from_rgb(255, 255, 100),
                    );
                    self.painter.paint_solid_circle(
                        Circle {
                            pos: board
                                .layout()
                                .drawing()
                                .primitive(destination)
                                .shape()
                                .center(),
                            r: 150.0,
                        },
                        egui::Color32::from_rgb(255, 255, 100),
                    );
                }
            }
        }
    }
}
