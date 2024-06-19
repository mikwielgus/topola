use std::sync::{Arc, Mutex};

use geo::point;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use topola::{
    autorouter::invoker::{Command, Invoker},
    board::mesadata::MesadataTrait,
    drawing::{
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::MakePrimitiveShape,
    },
    geometry::{shape::ShapeTrait, GenericNode},
    layout::{via::ViaWeight, zone::MakePolyShape},
    math::Circle,
    specctra::mesadata::SpecctraMesadata,
};

use crate::{
    app::{execute, SharedData},
    layers::Layers,
    overlay::Overlay,
    painter::Painter,
    top::Top,
};

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
        shared_data: Arc<Mutex<SharedData>>,
        maybe_invoker: &Option<Arc<Mutex<Invoker<SpecctraMesadata>>>>,
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

                if let Some(invoker_arc_mutex) = maybe_invoker {
                    if ctx.input(|i| i.pointer.any_click()) {
                        if top.is_placing_via {
                            let invoker_arc_mutex = invoker_arc_mutex.clone();

                            execute(async move {
                                let mut invoker = invoker_arc_mutex.lock().unwrap();
                                invoker.execute(
                                    Command::PlaceVia(ViaWeight {
                                        from_layer: 0,
                                        to_layer: 0,
                                        circle: Circle {
                                            pos: point! {x: latest_pos.x as f64, y: -latest_pos.y as f64},
                                            r: 10000.0,
                                        },
                                        maybe_net: Some(1234),
                                    }),
                                );
                            });
                        } else if let Some(overlay) = maybe_overlay {
                            let invoker = invoker_arc_mutex.lock().unwrap();
                            overlay.click(
                                invoker.autorouter().board(),
                                point! {x: latest_pos.x as f64, y: -latest_pos.y as f64},
                            );
                        }
                    }

                    if let (invoker, shared_data, Some(overlay)) = (
                        &invoker_arc_mutex.lock().unwrap(),
                        shared_data.lock().unwrap(),
                        maybe_overlay,
                    ) {
                        let board = invoker.autorouter().board();

                        if let Some(layers) = maybe_layers {
                            for i in (0..layers.visible.len()).rev() {
                                if layers.visible[i] {
                                    for primitive in board.layout().drawing().layer_primitive_nodes(i) {
                                        let shape = primitive.primitive(board.layout().drawing()).shape();

                                        let color = if shared_data.highlighteds.contains(&primitive)
                                            || overlay
                                                .selection()
                                                .contains_node(board, GenericNode::Primitive(primitive))
                                        {
                                            layers.highlight_colors[i]
                                        } else {
                                            layers.colors[i]
                                        };

                                        painter.paint_primitive(&shape, color);
                                    }

                                    for zone in board.layout().layer_zone_nodes(i) {
                                        let color = if overlay
                                            .selection()
                                            .contains_node(board, GenericNode::Compound(zone.into()))
                                        {
                                            layers.highlight_colors[i]
                                        } else {
                                            layers.colors[i]
                                        };

                                        painter.paint_polygon(&board.layout().zone(zone).shape().polygon, color)
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

                        if let Some(navmesh) = &shared_data.navmesh {
                            for edge in navmesh.graph().edge_references() {
                                let from = PrimitiveIndex::from(navmesh.graph().node_weight(edge.source()).unwrap().node)
                                    .primitive(board.layout().drawing())
                                    .shape()
                                    .center();
                                let to = PrimitiveIndex::from(navmesh.graph().node_weight(edge.target()).unwrap().node)
                                    .primitive(board.layout().drawing())
                                    .shape()
                                    .center();

                                let stroke = 'blk: {
                                    if let (Some(source_pos), Some(target_pos)) = (
                                        shared_data
                                            .path
                                            .iter()
                                            .position(|node| *node == navmesh.graph().node_weight(edge.source()).unwrap().node),
                                        shared_data
                                            .path
                                            .iter()
                                            .position(|node| *node == navmesh.graph().node_weight(edge.target()).unwrap().node),
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

                        for ghost in shared_data.ghosts.iter() {
                            painter.paint_primitive(&ghost, egui::Color32::from_rgb(75, 75, 150));
                        }

                        if let (Some(from), Some(to)) = (shared_data.from, shared_data.to) {
                            painter.paint_dot(
                                Circle {
                                    pos: board.layout().drawing().primitive(from).shape().center(),
                                    r: 20.0,
                                },
                                egui::Color32::from_rgb(255, 255, 100),
                            );
                            painter.paint_dot(
                                Circle {
                                    pos: board.layout().drawing().primitive(to).shape().center(),
                                    r: 20.0,
                                },
                                egui::Color32::from_rgb(255, 255, 100),
                            );
                        }
                    }
                }

                viewport_rect
            })
        }).inner.inner
    }
}
