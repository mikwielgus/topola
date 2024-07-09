use geo::point;
use petgraph::{
    data::DataMap,
    visit::{EdgeRef, IntoEdgeReferences},
};
use topola::{
    autorouter::invoker::{
        Command, ExecuteWithStatus, GetGhosts, GetMaybeNavmesh, GetMaybeTrace, GetObstacles,
        Invoker,
    },
    board::mesadata::AccessMesadata,
    drawing::{
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::MakePrimitiveShape,
    },
    geometry::{shape::AccessShape, GenericNode},
    layout::{poly::MakePolyShape, via::ViaWeight},
    math::Circle,
    specctra::mesadata::SpecctraMesadata,
};

use crate::{app::execute, layers::Layers, overlay::Overlay, painter::Painter, top::Top};

pub struct Viewport {
    pub from_rect: egui::emath::Rect,
}

impl Viewport {
    pub fn new() -> Self {
        Self {
            from_rect: egui::Rect::from_x_y_ranges(0.0..=1000000.0, 0.0..=500000.0),
        }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        top: &Top,
        maybe_invoker: &mut Option<Invoker<SpecctraMesadata>>,
        maybe_execute: &mut Option<ExecuteWithStatus>,
        maybe_overlay: &mut Option<Overlay>,
        maybe_layers: &Option<Layers>,
    ) -> egui::Rect {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                ui.ctx().request_repaint();

                let desired_size = ui.available_width() * egui::vec2(1.0, 0.5);
                let (_id, viewport_rect) = ui.allocate_space(desired_size);

                let old_transform =
                    egui::emath::RectTransform::from_to(self.from_rect, viewport_rect);
                let latest_pos = old_transform
                    .inverse()
                    .transform_pos(ctx.input(|i| i.pointer.latest_pos().unwrap_or_default()));

                let old_scale = old_transform.scale().x;
                self.from_rect = self.from_rect / ctx.input(|i| i.zoom_delta());

                let new_scale = egui::emath::RectTransform::from_to(self.from_rect, viewport_rect)
                    .scale()
                    .x;

                self.from_rect = self.from_rect.translate(
                    ctx.input(|i| latest_pos.to_vec2() * (new_scale - old_scale) / new_scale),
                );

                self.from_rect = self
                    .from_rect
                    .translate(ctx.input(|i| -i.raw_scroll_delta / new_scale));

                let transform = egui::emath::RectTransform::from_to(self.from_rect, viewport_rect);
                let mut painter = Painter::new(ui, transform);

                if let Some(ref mut invoker) = maybe_invoker {
                    if ctx.input(|i| i.pointer.any_click()) {
                        if top.is_placing_via {
                            invoker.execute(
                                Command::PlaceVia(ViaWeight {
                                    from_layer: 0,
                                    to_layer: 0,
                                    circle: Circle {
                                        pos: point! {x: latest_pos.x as f64, y: -latest_pos.y as f64},
                                        r: 100.0,
                                    },
                                    maybe_net: Some(1234),
                                }),
                            );
                        } else if let Some(overlay) = maybe_overlay {
                            overlay.click(
                                invoker.autorouter().board(),
                                point! {x: latest_pos.x as f64, y: -latest_pos.y as f64},
                            );
                        }
                    }

                    if let (Some(invoker), Some(overlay)) = (
                        maybe_invoker,
                        maybe_overlay,
                    ) {
                        let board = invoker.autorouter().board();

                        if let Some(layers) = maybe_layers {
                            for i in (0..layers.visible.len()).rev() {
                                if layers.visible[i] {
                                    for primitive in board.layout().drawing().layer_primitive_nodes(i) {
                                        let shape = primitive.primitive(board.layout().drawing()).shape();

                                        let color = if overlay
                                            .selection()
                                            .contains_node(board, GenericNode::Primitive(primitive))
                                        {
                                            layers.highlight_colors[i]
                                        } else {
                                            if let Some(execute) = maybe_execute {
                                                if execute.obstacles().contains(&primitive) {
                                                    layers.highlight_colors[i]
                                                } else {
                                                    layers.colors[i]
                                                }
                                            } else {
                                                layers.colors[i]
                                            }
                                        };

                                        painter.paint_primitive(&shape, color);
                                    }

                                    for poly in board.layout().layer_poly_nodes(i) {
                                        let color = if overlay
                                            .selection()
                                            .contains_node(board, GenericNode::Compound(poly.into()))
                                        {
                                            layers.highlight_colors[i]
                                        } else {
                                            layers.colors[i]
                                        };

                                        painter.paint_polygon(&board.layout().poly(poly).shape().polygon, color)
                                    }
                                }
                            }
                        }

                        if top.show_ratsnest {
                            for edge in overlay.ratsnest().graph().edge_references() {
                                let from = overlay
                                    .ratsnest()
                                    .graph()
                                    .node_weight(edge.source())
                                    .unwrap()
                                    .pos;
                                let to = overlay
                                    .ratsnest()
                                    .graph()
                                    .node_weight(edge.target())
                                    .unwrap()
                                    .pos;

                                painter.paint_edge(
                                    from,
                                    to,
                                    egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 90, 200)),
                                );
                            }
                        }

                        if top.show_navmesh {
                            if let Some(execute) = maybe_execute {
                                if let Some(navmesh) = execute.maybe_navmesh() {
                                    for edge in navmesh.edge_references() {
                                        let mut from = PrimitiveIndex::from(navmesh.node_weight(edge.source()).unwrap().node)
                                            .primitive(board.layout().drawing())
                                            .shape()
                                            .center();
                                        let mut to = PrimitiveIndex::from(navmesh.node_weight(edge.target()).unwrap().node)
                                            .primitive(board.layout().drawing())
                                            .shape()
                                            .center();

                                        if let Some(from_cw) = navmesh.node_weight(edge.source()).unwrap().maybe_cw {
                                            if from_cw {
                                                from -= [0.0, 150.0].into();
                                            } else {
                                                from += [0.0, 150.0].into();
                                            }
                                        }

                                        if let Some(to_cw) = navmesh.node_weight(edge.target()).unwrap().maybe_cw {
                                            if to_cw {
                                                to -= [0.0, 150.0].into();
                                            } else {
                                                to += [0.0, 150.0].into();
                                            }
                                        }

                                        let stroke = 'blk: {
                                            if let (Some(source_pos), Some(target_pos)) = (
                                                execute.maybe_trace().map(|trace|
                                                    trace.path
                                                    .iter()
                                                    .position(|node| *node == edge.source())).flatten(),
                                                execute.maybe_trace().map(|trace|
                                                    trace.path
                                                    .iter()
                                                    .position(|node| *node == edge.target())).flatten(),
                                            ) {
                                                if target_pos == source_pos + 1
                                                    || source_pos == target_pos + 1
                                                {
                                                    break 'blk egui::Stroke::new(
                                                        5.0,
                                                        egui::Color32::from_rgb(250, 250, 0),
                                                    );
                                                }
                                            }

                                            egui::Stroke::new(1.0, egui::Color32::from_rgb(125, 125, 125))
                                        };

                                        painter.paint_edge(from, to, stroke);
                                    }
                                }
                            }
                        }

                        if let Some(execute) = maybe_execute {
                            for ghost in execute.ghosts().iter() {
                                painter.paint_primitive(&ghost, egui::Color32::from_rgb(75, 75, 150));
                            }

                            if let Some(navmesh) = execute.maybe_navmesh() {
                                if let (origin, destination) = (navmesh.origin(), navmesh.destination()) {
                                    painter.paint_dot(
                                        Circle {
                                            pos: board.layout().drawing().primitive(origin).shape().center(),
                                            r: 150.0,
                                        },
                                        egui::Color32::from_rgb(255, 255, 100),
                                    );
                                    painter.paint_dot(
                                        Circle {
                                            pos: board.layout().drawing().primitive(destination).shape().center(),
                                            r: 150.0,
                                        },
                                        egui::Color32::from_rgb(255, 255, 100),
                                    );
                                }
                            }
                        }
                    }
                }

                viewport_rect
            })
        }).inner.inner
    }
}
