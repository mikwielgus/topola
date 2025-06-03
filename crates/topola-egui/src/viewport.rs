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
        invoker::{
            GetGhosts, GetMaybeAstarStepper, GetMaybeNavcord, GetNavmeshDebugTexts, GetObstacles,
        },
    },
    board::AccessMesadata,
    drawing::{
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::MakePrimitiveShape,
    },
    geometry::{shape::AccessShape, GenericNode},
    graph::MakeRef,
    layout::{poly::MakePolygon, via::ViaWeight},
    math::{Circle, RotationSense},
    router::navmesh::NavnodeIndex,
};

use crate::{config::Config, menu_bar::MenuBar, painter::Painter, workspace::Workspace};

pub struct Viewport {
    pub transform: egui::emath::TSTransform,
    /// how much should a single arrow key press scroll
    pub kbd_scroll_delta_factor: f32,
    pub scheduled_zoom_to_fit: bool,
}

impl Viewport {
    pub fn new() -> Self {
        Self {
            transform: egui::emath::TSTransform::new([0.0, 0.0].into(), 0.01),
            kbd_scroll_delta_factor: 5.0,
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
                    // TODO: only request re-render if anything changed
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

                    // disable built-in behavior of arrow keys
                    if response.has_focus() {
                        response.ctx.memory_mut(|m| {
                            // we are only allowed to modify the focus lock filter if we have focus
                            m.set_focus_lock_filter(
                                id,
                                egui::EventFilter {
                                    horizontal_arrows: true,
                                    vertical_arrows: true,
                                    ..Default::default()
                                },
                            );
                        });
                    }

                    self.transform.translation += ctx.input_mut(|i| {
                        // handle scrolling
                        let mut scroll_delta = core::mem::take(&mut i.smooth_scroll_delta);

                        // arrow keys
                        let kbd_sdf = self.kbd_scroll_delta_factor;
                        let mut pressed = |key| {
                            i.consume_shortcut(&egui::KeyboardShortcut::new(
                                egui::Modifiers::SHIFT,
                                key,
                            ))
                        };
                        use egui::Key;
                        scroll_delta.y += if pressed(Key::ArrowDown) {
                            kbd_sdf
                        } else if pressed(Key::ArrowUp) {
                            -kbd_sdf
                        } else {
                            0.0
                        };
                        scroll_delta.x += if pressed(Key::ArrowRight) {
                            kbd_sdf
                        } else if pressed(Key::ArrowLeft) {
                            -kbd_sdf
                        } else {
                            0.0
                        };

                        scroll_delta
                    });

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

                                    painter.paint_polygon(&poly.ref_(board.layout()).shape(), color)
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
                                if let Some(astar) = activity.maybe_astar() {
                                    let navmesh = astar.graph();

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

                                        if let Some(from_sense) =
                                            navmesh.node_weight(edge.source()).unwrap().maybe_sense
                                        {
                                            from += match from_sense {
                                                RotationSense::Counterclockwise => {
                                                    [0.0, 150.0].into()
                                                }
                                                RotationSense::Clockwise => [-0.0, -150.0].into(),
                                            };
                                        }

                                        if let Some(to_sense) =
                                            navmesh.node_weight(edge.target()).unwrap().maybe_sense
                                        {
                                            to += match to_sense {
                                                RotationSense::Counterclockwise => {
                                                    [0.0, 150.0].into()
                                                }
                                                RotationSense::Clockwise => [-0.0, -150.0].into(),
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

                                        if let Some(text) = activity
                                            .navedge_debug_text((edge.source(), edge.target()))
                                        {
                                            painter.paint_text(
                                                (from + to) / 2.0,
                                                egui::Align2::LEFT_BOTTOM,
                                                text,
                                                egui::Color32::from_rgb(255, 255, 255),
                                            );
                                        }
                                    }

                                    for index in navmesh.graph().node_indices() {
                                        let navnode = NavnodeIndex(index);
                                        let mut pos = PrimitiveIndex::from(
                                            navmesh.node_weight(navnode).unwrap().node,
                                        )
                                        .primitive(board.layout().drawing())
                                        .shape()
                                        .center();

                                        pos += match navmesh
                                            .node_weight(navnode)
                                            .unwrap()
                                            .maybe_sense
                                        {
                                            Some(RotationSense::Counterclockwise) => {
                                                [0.0, 150.0].into()
                                            }
                                            Some(RotationSense::Clockwise) => [-0.0, -150.0].into(),
                                            None => [0.0, 0.0].into(),
                                        };

                                        //TODO "{astar.scores[index]} ({astar.estimate_scores[index]}) (...)"
                                        let score_text = astar
                                            .scores()
                                            .get(&navnode)
                                            .map_or_else(String::new, |s| format!("g={:.2}", s));
                                        let estimate_score_text = astar
                                            .estimate_scores()
                                            .get(&navnode)
                                            .map_or_else(String::new, |s| format!("(f={:.2})", s));
                                        let debug_text =
                                            activity.navnode_debug_text(navnode).unwrap_or("");

                                        painter.paint_text(
                                            pos,
                                            egui::Align2::LEFT_BOTTOM,
                                            &format!(
                                                "{} {} {}",
                                                score_text, estimate_score_text, debug_text
                                            ),
                                            egui::Color32::from_rgb(255, 255, 255),
                                        );
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

                            if let Some(ref navmesh) =
                                activity.maybe_astar().map(|astar| astar.graph())
                            {
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
