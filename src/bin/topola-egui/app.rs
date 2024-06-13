use futures::executor;
use geo::point;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    future::Future,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
};

use topola::{
    autorouter::{
        invoker::{Command, Execute, Invoker, InvokerStatus},
        Autorouter,
    },
    drawing::{
        dot::FixedDotIndex,
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::MakePrimitiveShape,
        rules::RulesTrait,
        Drawing, Infringement, LayoutException,
    },
    dsn::{design::DsnDesign, mesadata::DsnMesadata},
    geometry::{
        compound::CompoundManagerTrait,
        primitive::{BendShape, DotShape, PrimitiveShape, PrimitiveShapeTrait, SegShape},
        shape::ShapeTrait,
        GenericNode,
    },
    layout::{via::ViaWeight, zone::MakePolyShape, Layout},
    math::Circle,
    router::{
        draw::DrawException,
        navmesh::{Navmesh, NavmeshEdgeReference, NavvertexIndex},
        tracer::{Trace, Tracer},
        EmptyRouterObserver, RouterObserverTrait,
    },
};

use crate::{layers::Layers, overlay::Overlay, painter::Painter, top::Top};

#[derive(Debug, Default)]
pub struct SharedData {
    pub from: Option<FixedDotIndex>,
    pub to: Option<FixedDotIndex>,
    pub navmesh: Option<Navmesh>,
    pub path: Vec<NavvertexIndex>,
    pub ghosts: Vec<PrimitiveShape>,
    pub highlighteds: Vec<PrimitiveIndex>,
}

/// Deserialize/Serialize is needed to persist app state between restarts.
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct App {
    #[serde(skip)]
    overlay: Option<Overlay>,

    #[serde(skip)]
    invoker: Option<Arc<Mutex<Invoker<DsnMesadata>>>>,

    #[serde(skip)]
    shared_data: Arc<Mutex<SharedData>>,

    #[serde(skip)]
    text_channel: (Sender<String>, Receiver<String>),

    #[serde(skip)]
    from_rect: egui::emath::Rect,

    #[serde(skip)]
    top: Top,

    #[serde(skip)]
    layers: Option<Layers>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            overlay: None,
            invoker: None,
            shared_data: Default::default(),
            text_channel: channel(),
            from_rect: egui::Rect::from_x_y_ranges(0.0..=1000000.0, 0.0..=500000.0),
            top: Top::new(),
            layers: None,
        }
    }
}

impl App {
    /// Called once on start.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load previous app state if one exists.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

pub struct DebugRouterObserver {
    pub shared_data: Arc<Mutex<SharedData>>,
}

impl<R: RulesTrait + std::fmt::Debug> RouterObserverTrait<R> for DebugRouterObserver {
    fn on_rework(&mut self, tracer: &Tracer<R>, trace: &Trace) {
        //dbg!(_tracer, _trace);
        let mut shared_data = self.shared_data.lock().unwrap();
        shared_data.path = trace.path.clone();
        shared_data.ghosts = vec![];
        shared_data.highlighteds = vec![];
        std::thread::sleep_ms(500);
    }
    fn before_probe(&mut self, tracer: &Tracer<R>, trace: &Trace, edge: NavmeshEdgeReference) {
        //dbg!(_tracer, _trace, _edge);
        let mut shared_data = self.shared_data.lock().unwrap();
        shared_data.path = trace.path.clone();
        shared_data.path.push(edge.target());
        shared_data.ghosts = vec![];
        shared_data.highlighteds = vec![];
        std::thread::sleep_ms(100);
    }
    fn on_probe(
        &mut self,
        tracer: &Tracer<R>,
        trace: &Trace,
        edge: NavmeshEdgeReference,
        result: Result<(), DrawException>,
    ) {
        //dbg!(_tracer, _trace, _edge, _result);
        let mut shared_data = self.shared_data.lock().unwrap();
        let (ghosts, highlighteds, delay) = match result {
            Err(DrawException::CannotWrapAround(
                ..,
                LayoutException::Infringement(Infringement(shape1, infringee1)),
                LayoutException::Infringement(Infringement(shape2, infringee2)),
            )) => (vec![shape1, shape2], vec![infringee1, infringee2], 1500),
            _ => (vec![], vec![], 300),
        };

        shared_data.path = trace.path.clone();
        shared_data.ghosts = ghosts;
        shared_data.highlighteds = highlighteds;
        std::thread::sleep_ms(delay);
    }
    fn on_estimate(&mut self, _tracer: &Tracer<R>, _vertex: NavvertexIndex) {
        //dbg!(_tracer, _vertex);
    }
}

impl eframe::App for App {
    /// Called to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI has to be repainted.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if cfg!(target_arch = "wasm32") {
            if let Ok(file_contents) = self.text_channel.1.try_recv() {
                let design = DsnDesign::load_from_string(file_contents).unwrap();
                let board = design.make_board();
                self.overlay = Some(Overlay::new(&board).unwrap());
                self.layers = Some(Layers::new(&board));
                self.invoker = Some(Arc::new(Mutex::new(Invoker::new(
                    Autorouter::new(board).unwrap(),
                ))));
            }
        } else {
            if let Ok(path) = self.text_channel.1.try_recv() {
                let design = DsnDesign::load_from_file(&path).unwrap();
                let board = design.make_board();
                self.overlay = Some(Overlay::new(&board).unwrap());
                self.layers = Some(Layers::new(&board));
                self.invoker = Some(Arc::new(Mutex::new(Invoker::new(
                    Autorouter::new(board).unwrap(),
                ))));
            }
        }

        self.top.update(
            ctx,
            self.shared_data.clone(),
            self.text_channel.0.clone(),
            &self.invoker,
            &self.overlay,
        );

        if let Some(ref mut layers) = self.layers {
            if let Some(invoker_arc_mutex) = &self.invoker {
                layers.update(ctx, invoker_arc_mutex.lock().unwrap().autorouter().board());
            }
        }

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

                if let Some(invoker_arc_mutex) = &self.invoker {
                    if ctx.input(|i| i.pointer.any_click()) {
                        if self.top.is_placing_via {
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
                                    &mut EmptyRouterObserver,
                                );
                            });
                        } else if let Some(overlay) = &mut self.overlay {
                            let invoker = invoker_arc_mutex.lock().unwrap();
                            overlay.click(
                                invoker.autorouter().board(),
                                point! {x: latest_pos.x as f64, y: -latest_pos.y as f64},
                            );
                        }
                    }

                    if let (invoker, shared_data, Some(overlay)) = (
                        &invoker_arc_mutex.lock().unwrap(),
                        self.shared_data.lock().unwrap(),
                        &mut self.overlay,
                    ) {
                        let board = invoker.autorouter().board();

                        for primitive in board.layout().drawing().layer_primitive_nodes(1) {
                            let shape = primitive.primitive(board.layout().drawing()).shape();

                            let color = if shared_data.highlighteds.contains(&primitive)
                                || overlay
                                    .selection()
                                    .contains_node(board, GenericNode::Primitive(primitive))
                            {
                                egui::Color32::from_rgb(100, 100, 255)
                            } else {
                                egui::Color32::from_rgb(52, 52, 200)
                            };
                            painter.paint_primitive(&shape, color);
                        }

                        for zone in board.layout().layer_zone_nodes(1) {
                            let color = if overlay
                                .selection()
                                .contains_node(board, GenericNode::Compound(zone.into()))
                            {
                                egui::Color32::from_rgb(100, 100, 255)
                            } else {
                                egui::Color32::from_rgb(52, 52, 200)
                            };
                            painter.paint_polygon(&board.layout().zone(zone).shape().polygon, color)
                        }

                        for primitive in board.layout().drawing().layer_primitive_nodes(0) {
                            let shape = primitive.primitive(board.layout().drawing()).shape();

                            let color = if shared_data.highlighteds.contains(&primitive)
                                || overlay
                                    .selection()
                                    .contains_node(board, GenericNode::Primitive(primitive))
                            {
                                egui::Color32::from_rgb(255, 100, 100)
                            } else {
                                egui::Color32::from_rgb(200, 52, 52)
                            };
                            painter.paint_primitive(&shape, color);
                        }

                        for zone in board.layout().layer_zone_nodes(0) {
                            let color = if overlay
                                .selection()
                                .contains_node(board, GenericNode::Compound(zone.into()))
                            {
                                egui::Color32::from_rgb(255, 100, 100)
                            } else {
                                egui::Color32::from_rgb(200, 52, 52)
                            };
                            painter.paint_polygon(&board.layout().zone(zone).shape().polygon, color)
                        }

                        if self.top.show_ratsnest {
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
                            for edge in navmesh.edge_references() {
                                let from = edge
                                    .source()
                                    .primitive(board.layout().drawing())
                                    .shape()
                                    .center();
                                let to = edge
                                    .target()
                                    .primitive(board.layout().drawing())
                                    .shape()
                                    .center();

                                let stroke = 'blk: {
                                    if let (Some(source_pos), Some(target_pos)) = (
                                        shared_data
                                            .path
                                            .iter()
                                            .position(|node| *node == edge.source()),
                                        shared_data
                                            .path
                                            .iter()
                                            .position(|node| *node == edge.target()),
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

                        //unreachable!();
                    }
                }
            })
        });

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    std::thread::spawn(move || futures::executor::block_on(f));
}

#[cfg(target_arch = "wasm32")]
pub fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn channel_text(file_handle: rfd::FileHandle) -> String {
    file_handle.path().to_str().unwrap().to_string()
}

#[cfg(target_arch = "wasm32")]
pub async fn channel_text(file_handle: rfd::FileHandle) -> String {
    std::str::from_utf8(&file_handle.read().await)
        .unwrap()
        .to_string()
}
