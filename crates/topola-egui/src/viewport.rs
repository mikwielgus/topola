// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::point;
use petgraph::{
    data::DataMap,
    visit::{EdgeRef, IntoEdgeReferences},
};
use rstar::{Envelope, AABB};
use topola::{
    autorouter::{
        execution::Command,
        invoker::{GetGhosts, GetMaybeNavcord, GetMaybeNavmesh, GetObstacles},
    },
    board::AccessMesadata,
    drawing::{
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::MakePrimitiveShape,
    },
    geometry::{shape::AccessShape, GenericNode},
    layout::{poly::MakePolygon, via::ViaWeight},
    math::Circle,
};

use crate::{config::Config, menu_bar::MenuBar, painter::Painter, workspace::Workspace};

pub struct Viewport {
    pub transform: egui::emath::TSTransform,
    pub scheduled_zoom_to_fit: bool,
}

impl Viewport {
    pub fn new() -> Self {
        Self {
            transform: egui::emath::TSTransform::new([0.0, 0.0].into(), 0.01),
            scheduled_zoom_to_fit: false,
        }
    }

    pub fn update(
        &mut self,
        config: &Config,
        ctx: &egui::Context,
        menu_bar: &MenuBar,
        maybe_workspace: Option<&mut Workspace>,
    ) -> egui::Rect {
        egui::CentralPanel::default()
            .show(ctx, |ui| {
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    ui.ctx().request_repaint();

                    let (id, viewport_rect) = ui.allocate_space(ui.available_size());
                    let response = ui.interact(viewport_rect, id, egui::Sense::click_and_drag());
                    // NOTE: we use `interact_pos` instead of `latest_pos` to handle "pointer gone"
                    // events more graceful
                    let latest_pos = self.transform.inverse()
                        * (response.interact_pointer_pos().unwrap_or_else(|| {
                            ctx.input(|i| i.pointer.interact_pos().unwrap_or_default())
                        }));

                    let old_scaling = self.transform.scaling;
                    self.transform.scaling *= ctx.input(|i| i.zoom_delta());

                    self.transform.translation +=
                        latest_pos.to_vec2() * (old_scaling - self.transform.scaling);
                    self.transform.translation += ctx.input(|i| i.smooth_scroll_delta);

                    let mut painter = Painter::new(ui, self.transform, menu_bar.show_bboxes);

                    if let Some(workspace) = maybe_workspace {
                        let layers = &mut workspace.appearance_panel;
                        let overlay = &mut workspace.overlay;
                        let latest_point = point! {x: latest_pos.x as f64, y: -latest_pos.y as f64};
                        let board = workspace.interactor.invoker().autorouter().board();

                        if response.clicked_by(egui::PointerButton::Primary) {
                            if menu_bar.is_placing_via {
                                workspace.interactor.execute(Command::PlaceVia(ViaWeight {
                                    from_layer: 0,
                                    to_layer: 0,
                                    circle: Circle {
                                        pos: latest_point,
                                        r: menu_bar
                                            .autorouter_options
                                            .router_options
                                            .routed_band_width
                                            / 2.0,
                                    },
                                    maybe_net: Some(1234),
                                }));
                            } else {
                                overlay.click(board, layers, latest_point);
                            }
                        } else if response.drag_started_by(egui::PointerButton::Primary) {
                            overlay.drag_start(
                                board,
                                layers,
                                latest_point,
                                &response.ctx.input(|i| i.modifiers),
                            );
                        } else if response.drag_stopped_by(egui::PointerButton::Primary) {
                            overlay.drag_stop(board, layers, latest_point);
                        } else if let Some((_, bsk, cur_bbox)) =
                            overlay.get_bbox_reselect(latest_point)
                        {
                            use topola::autorouter::selection::BboxSelectionKind;
                            painter.paint_bbox_with_color(
                                cur_bbox,
                                match bsk {
                                    BboxSelectionKind::CompletelyInside => egui::Color32::YELLOW,
                                    BboxSelectionKind::MerelyIntersects => egui::Color32::BLUE,
                                },
                            );
                        }

                        let board = workspace.interactor.invoker().autorouter().board();

                        for i in (0..layers.visible.len()).rev() {
                            if layers.visible[i] {
                                for primitive in board.layout().drawing().layer_primitive_nodes(i) {
                                    let shape =
                                        primitive.primitive(board.layout().drawing()).shape();

                                    let color = if overlay
                                        .selection()
                                        .contains_node(board, GenericNode::Primitive(primitive))
                                    {
                                        config
                                            .colors(ctx)
                                            .layers
                                            .color(board.layout().rules().layer_layername(i))
                                            .highlighted
                                    } else if let Some(activity) =
                                        &mut workspace.interactor.maybe_activity()
                                    {
                                        if activity.obstacles().contains(&primitive) {
                                            config
                                                .colors(ctx)
                                                .layers
                                                .color(board.layout().rules().layer_layername(i))
                                                .highlighted
                                        } else {
                                            config
                                                .colors(ctx)
                                                .layers
                                                .color(board.layout().rules().layer_layername(i))
                                                .normal
                                        }
                                    } else {
                                        config
                                            .colors(ctx)
                                            .layers
                                            .color(board.layout().rules().layer_layername(i))
                                            .normal
                                    };

                                    painter.paint_primitive(&shape, color);
                                }

                                for poly in board.layout().layer_poly_nodes(i) {
                                    let color = if overlay
                                        .selection()
                                        .contains_node(board, GenericNode::Compound(poly.into()))
                                    {
                                        config
                                            .colors(ctx)
                                            .layers
                                            .color(board.layout().rules().layer_layername(i))
                                            .highlighted
                                    } else {
                                        config
                                            .colors(ctx)
                                            .layers
                                            .color(board.layout().rules().layer_layername(i))
                                            .normal
                                    };

                                    painter.paint_polygon(&board.layout().poly(poly).shape(), color)
                                }
                            }
                        }

                        if menu_bar.show_ratsnest {
                            let graph = overlay.ratsnest().graph();
                            for edge in graph.edge_references() {
                                let from = graph.node_weight(edge.source()).unwrap().pos;
                                let to = graph.node_weight(edge.target()).unwrap().pos;

                                painter.paint_edge(
                                    from,
                                    to,
                                    egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 90, 200)),
                                );
                            }
                        }

                        if menu_bar.show_navmesh {
                            if let Some(activity) = workspace.interactor.maybe_activity() {
                                if let Some(navmesh) = activity.maybe_navmesh() {
                                    for edge in navmesh.edge_references() {
                                        let mut from = PrimitiveIndex::from(
                                            navmesh.node_weight(edge.source()).unwrap().node,
                                        )
                                        .primitive(board.layout().drawing())
                                        .shape()
                                        .center();
                                        let mut to = PrimitiveIndex::from(
                                            navmesh.node_weight(edge.target()).unwrap().node,
                                        )
                                        .primitive(board.layout().drawing())
                                        .shape()
                                        .center();

                                        if let Some(from_cw) =
                                            navmesh.node_weight(edge.source()).unwrap().maybe_cw
                                        {
                                            if from_cw {
                                                from -= [0.0, 150.0].into();
                                            } else {
                                                from += [0.0, 150.0].into();
                                            }
                                        }

                                        if let Some(to_cw) =
                                            navmesh.node_weight(edge.target()).unwrap().maybe_cw
                                        {
                                            if to_cw {
                                                to -= [0.0, 150.0].into();
                                            } else {
                                                to += [0.0, 150.0].into();
                                            }
                                        }

                                        let stroke = 'blk: {
                                            if let Some(navcord) = activity.maybe_navcord() {
                                                if let (Some(source_pos), Some(target_pos)) =
                                                    (
                                                        navcord.path.iter().position(|node| {
                                                            *node == edge.source()
                                                        }),
                                                        navcord.path.iter().position(|node| {
                                                            *node == edge.target()
                                                        }),
                                                    )
                                                {
                                                    if target_pos == source_pos + 1
                                                        || source_pos == target_pos + 1
                                                    {
                                                        break 'blk egui::Stroke::new(
                                                            5.0,
                                                            egui::Color32::from_rgb(250, 250, 0),
                                                        );
                                                    }
                                                }
                                            }

                                            egui::Stroke::new(
                                                1.0,
                                                egui::Color32::from_rgb(125, 125, 125),
                                            )
                                        };

                                        painter.paint_edge(from, to, stroke);
                                    }
                                }
                            }
                        }

                        if menu_bar.show_topo_navmesh {
                            if let Some(navmesh) = workspace.overlay.planar_incr_navmesh() {
                                // calculate dual node position approximations
                                use std::collections::BTreeMap;
                                use topola::geometry::shape::AccessShape;
                                use topola::router::planar_incr_embed::NavmeshIndex;
                                let mut map = BTreeMap::new();
                                let resolve_primal = |p: &topola::layout::NodeIndex| {
                                    board.layout().node_shape(*p).center()
                                };

                                for (nidx, node) in &*navmesh.nodes {
                                    if let NavmeshIndex::Dual(didx) = nidx {
                                        map.insert(
                                            didx,
                                            geo::point! { x: node.pos.x, y: node.pos.y },
                                        );
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
                                    let edge_len = navmesh.edge_paths[edge.1].len();
                                    use egui::Color32;
                                    let stroke = if edge_len == 0 {
                                        egui::Stroke::new(
                                            1.0,
                                            if got_primal {
                                                Color32::from_rgb(255, 175, 0)
                                            } else {
                                                Color32::from_rgb(159, 255, 33)
                                            },
                                        )
                                    } else {
                                        egui::Stroke::new(
                                            1.5 + (edge_len as f32).atan(),
                                            Color32::from_rgb(250, 250, 0),
                                        )
                                    };
                                    painter.paint_edge(a_pos, b_pos, stroke);
                                }
                            }
                        }

                        if menu_bar.show_bboxes {
                            let root_bbox3d = board.layout().drawing().rtree().root().envelope();
                            let root_bbox = AABB::<[f64; 2]>::from_corners(
                                [root_bbox3d.lower()[0], root_bbox3d.lower()[1]],
                                [root_bbox3d.upper()[0], root_bbox3d.upper()[1]],
                            );
                            painter.paint_bbox(root_bbox);
                        }

                        if let Some(activity) = workspace.interactor.maybe_activity() {
                            for ghost in activity.ghosts().iter() {
                                painter
                                    .paint_primitive(ghost, egui::Color32::from_rgb(75, 75, 150));
                            }

                            if let Some(navmesh) = activity.maybe_navmesh() {
                                if menu_bar.show_origin_destination {
                                    let (origin, destination) =
                                        (navmesh.origin(), navmesh.destination());
                                    painter.paint_dot(
                                        Circle {
                                            pos: board
                                                .layout()
                                                .drawing()
                                                .primitive(origin)
                                                .shape()
                                                .center(),
                                            r: 150.0,
                                        },
                                        egui::Color32::from_rgb(255, 255, 100),
                                    );
                                    painter.paint_dot(
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

                        if self.scheduled_zoom_to_fit {
                            let root_bbox = workspace
                                .interactor
                                .invoker()
                                .autorouter()
                                .board()
                                .layout()
                                .drawing()
                                .rtree()
                                .root()
                                .envelope();

                            let root_bbox_width = root_bbox.upper()[0] - root_bbox.lower()[0];
                            let root_bbox_height = root_bbox.upper()[1] - root_bbox.lower()[1];

                            self.transform.scaling = 0.8
                                * if root_bbox_width / root_bbox_height
                                    >= (viewport_rect.width() as f64)
                                        / (viewport_rect.height() as f64)
                                {
                                    viewport_rect.width() / root_bbox_width as f32
                                } else {
                                    viewport_rect.height() / root_bbox_height as f32
                                };

                            self.transform.translation = egui::Vec2::new(
                                viewport_rect.center()[0],
                                viewport_rect.center()[1],
                            ) - (self.transform.scaling
                                * egui::Pos2::new(
                                    root_bbox.center()[0] as f32,
                                    -root_bbox.center()[1] as f32,
                                ))
                            .to_vec2();
                        }

                        self.scheduled_zoom_to_fit = false;
                    }

                    viewport_rect
                })
            })
            .inner
            .inner
    }
}
