use geo::point;
use petgraph::{
    data::DataMap,
    visit::{EdgeRef, IntoEdgeReferences},
};
use rstar::{Envelope, AABB};
use topola::{
    autorouter::{
        execution::Command,
        invoker::{GetGhosts, GetMaybeNavmesh, GetMaybeTrace, GetObstacles, Invoker},
    },
    drawing::{
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::MakePrimitiveShape,
    },
    geometry::{shape::AccessShape, GenericNode},
    layout::{poly::MakePolyShape, via::ViaWeight},
    math::Circle,
    specctra::mesadata::SpecctraMesadata,
};

use crate::{
    activity::ActivityStepperWithStatus, layers::Layers, menu_bar::MenuBar, overlay::Overlay,
    painter::Painter, workspace::Workspace,
};

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
        ctx: &egui::Context,
        top: &MenuBar,
        mut maybe_workspace: Option<&mut Workspace>,
    ) -> egui::Rect {
        let viewport_rect = self.paint(ctx, top, maybe_workspace.as_deref_mut());

        if self.scheduled_zoom_to_fit {
            let mut maybe_invoker = maybe_workspace.as_ref().map(|w| w.invoker.lock().unwrap());
            self.zoom_to_fit(maybe_invoker.as_deref_mut(), &viewport_rect);
        }

        viewport_rect
    }

    pub fn paint(
        &mut self,
        ctx: &egui::Context,
        top: &MenuBar,
        maybe_workspace: Option<&mut Workspace>,
    ) -> egui::Rect {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                ui.ctx().request_repaint();

                let (_id, viewport_rect) = ui.allocate_space(ui.available_size());
                let latest_pos = self.transform.inverse() * (ctx.input(|i| i.pointer.latest_pos().unwrap_or_default()));

                let old_scaling = self.transform.scaling;
                self.transform.scaling *= ctx.input(|i| i.zoom_delta());

                self.transform.translation += latest_pos.to_vec2() * (old_scaling - self.transform.scaling);
                self.transform.translation += ctx.input(|i| i.smooth_scroll_delta);

                let mut painter = Painter::new(ui, self.transform, top.show_bboxes);

                if let Some(workspace) = maybe_workspace {
                    let mut invoker = workspace.invoker.lock().unwrap();
                    let layers = &mut workspace.layers;
                    let overlay = &mut workspace.overlay;

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
                        } else {
                            overlay.click(
                                invoker.autorouter().board(),
                                point! {x: latest_pos.x as f64, y: -latest_pos.y as f64},
                            );
                        }
                    }

                    let board = invoker.autorouter().board();
                    for i in (0..layers.visible.len()).rev() {
                        if layers.visible[i] {
                            for primitive in board.layout().drawing().layer_primitive_nodes(i) {
                                let shape = primitive.primitive(board.layout().drawing()).shape();

                                let color = if overlay
                                    .selection()
                                    .contains_node(board, GenericNode::Primitive(primitive))
                                {
                                    layers.highlight_colors[i]
                                } else if let Some(activity) = &mut workspace.maybe_activity {
                                    if activity.obstacles().contains(&primitive) {
                                        layers.highlight_colors[i]
                                    } else {
                                        layers.colors[i]
                                    }
                                } else {
                                    layers.colors[i]
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

                    if top.show_ratsnest {
                        let graph = overlay.ratsnest().graph();
                        for edge in graph.edge_references() {
                            let from = graph
                                .node_weight(edge.source())
                                .unwrap()
                                .pos;
                            let to = graph
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
                        if let Some(activity) = &mut workspace.maybe_activity {
                            if let Some(navmesh) = activity.maybe_navmesh() {
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
                                            activity.maybe_trace().map(|trace|
                                                trace.path
                                                .iter()
                                                .position(|node| *node == edge.source())).flatten(),
                                            activity.maybe_trace().map(|trace|
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

                    if top.show_bboxes {
                        let root_bbox3d = board.layout().drawing().rtree().root().envelope();

                        let root_bbox = AABB::<[f64; 2]>::from_corners([root_bbox3d.lower()[0], root_bbox3d.lower()[1]].into(), [root_bbox3d.upper()[0], root_bbox3d.upper()[1]].into());
                        painter.paint_bbox(root_bbox);
                    }

                    if let Some(activity) = &mut workspace.maybe_activity {
                        for ghost in activity.ghosts().iter() {
                            painter.paint_primitive(&ghost, egui::Color32::from_rgb(75, 75, 150));
                        }

                        if let Some(navmesh) = activity.maybe_navmesh() {
                            if top.show_origin_destination {
                                let (origin, destination) = (navmesh.origin(), navmesh.destination());
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

                viewport_rect
            })
        }).inner.inner
    }

    fn zoom_to_fit(
        &mut self,
        maybe_invoker: Option<&mut Invoker<SpecctraMesadata>>,
        viewport_rect: &egui::Rect,
    ) {
        if self.scheduled_zoom_to_fit {
            if let Some(invoker) = maybe_invoker {
                let root_bbox = invoker
                    .autorouter()
                    .board()
                    .layout()
                    .drawing()
                    .rtree()
                    .root()
                    .envelope();

                let root_bbox_width = root_bbox.upper()[0] - root_bbox.lower()[0];
                let root_bbox_height = root_bbox.upper()[1] - root_bbox.lower()[1];

                if root_bbox_width / root_bbox_height
                    >= (viewport_rect.width() as f64) / (viewport_rect.height() as f64)
                {
                    self.transform.scaling = 0.8 * viewport_rect.width() / root_bbox_width as f32;
                } else {
                    self.transform.scaling = 0.8 * viewport_rect.height() / root_bbox_height as f32;
                }

                self.transform.translation = egui::Vec2::new(
                    viewport_rect.center()[0] as f32,
                    viewport_rect.center()[1] as f32,
                ) - (self.transform.scaling
                    * egui::Pos2::new(root_bbox.center()[0] as f32, -root_bbox.center()[1] as f32))
                .to_vec2();
            }
        }

        self.scheduled_zoom_to_fit = false;
    }
}
