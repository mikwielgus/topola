use futures::executor;
use geo::point;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use std::{
    future::Future,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
};

use topola::{
    autorouter::{
        invoker::{Command, Execute, Invoker},
        Autorouter,
    },
    drawing::{
        dot::FixedDotIndex,
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::MakePrimitiveShape,
        rules::RulesTrait,
        Drawing, Infringement, LayoutException,
    },
    dsn::{design::DsnDesign, rules::DsnRules},
    geometry::{
        compound::CompoundManagerTrait,
        primitive::{BendShape, DotShape, PrimitiveShape, PrimitiveShapeTrait, SegShape},
        shape::ShapeTrait,
        GenericNode,
    },
    layout::{zone::MakePolyShape, Layout},
    math::Circle,
    router::{
        draw::DrawException,
        navmesh::{Navmesh, NavmeshEdgeReference, VertexIndex},
        tracer::{Trace, Tracer},
        RouterObserverTrait,
    },
};

use crate::{overlay::Overlay, painter::Painter};

#[derive(Debug, Default)]
struct SharedData {
    pub from: Option<FixedDotIndex>,
    pub to: Option<FixedDotIndex>,
    pub navmesh: Option<Navmesh>,
    pub path: Vec<VertexIndex>,
    pub ghosts: Vec<PrimitiveShape>,
    pub highlighteds: Vec<PrimitiveIndex>,
}

/// Deserialize/Serialize is needed to persist app state between restarts.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct App {
    #[serde(skip)]
    overlay: Option<Overlay>,

    #[serde(skip)]
    layout: Option<Arc<Mutex<Layout<DsnRules>>>>,

    #[serde(skip)]
    shared_data: Arc<Mutex<SharedData>>,

    #[serde(skip)]
    text_channel: (Sender<String>, Receiver<String>),

    #[serde(skip)]
    from_rect: egui::emath::Rect,
}

impl Default for App {
    fn default() -> Self {
        Self {
            overlay: None,
            layout: None,
            shared_data: Default::default(),
            text_channel: channel(),
            from_rect: egui::Rect::from_x_y_ranges(0.0..=1000000.0, 0.0..=500000.0),
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

struct DebugRouterObserver {
    shared_data: Arc<Mutex<SharedData>>,
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
    fn on_estimate(&mut self, _tracer: &Tracer<R>, _vertex: VertexIndex) {
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
                let layout = design.make_layout();
                self.overlay = Some(Overlay::new(&layout).unwrap());
                self.layout = Some(Arc::new(Mutex::new(layout)));
            }
        } else {
            if let Ok(path) = self.text_channel.1.try_recv() {
                let design = DsnDesign::load_from_file(&path).unwrap();
                let layout = design.make_layout();
                self.overlay = Some(Overlay::new(&layout).unwrap());
                self.layout = Some(Arc::new(Mutex::new(layout)));
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        // `Context` is cheap to clone as it's wrapped in an `Arc`.
                        let ctx = ui.ctx().clone();
                        // NOTE: On Linux, this requires Zenity to be installed on your system.
                        let sender = self.text_channel.0.clone();
                        let task = rfd::AsyncFileDialog::new().pick_file();

                        execute(async move {
                            let maybe_file_handle = task.await;

                            if let Some(file_handle) = maybe_file_handle {
                                let _ = sender.send(channel_text(file_handle).await);
                                ctx.request_repaint();
                            }
                        });
                    }

                    // "Quit" button wouldn't work on a Web page.
                    if !cfg!(target_arch = "wasm32") {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                });

                ui.separator();

                if ui.button("Autoroute").clicked() {
                    if let (Some(layout_arc_mutex), Some(overlay)) = (&self.layout, &self.overlay) {
                        let layout = layout_arc_mutex.clone();
                        let shared_data_arc_mutex = self.shared_data.clone();
                        let selection = overlay.selection().clone();

                        execute(async move {
                            let mut invoker = Invoker::new(Autorouter::new(layout).unwrap());
                            let mut execute = invoker.execute_walk(&Command::Autoroute(selection));

                            if let Execute::Autoroute(ref mut autoroute) = execute {
                                let from = autoroute.navmesh().as_ref().unwrap().from();
                                let to = autoroute.navmesh().as_ref().unwrap().to();

                                {
                                    let mut shared_data = shared_data_arc_mutex.lock().unwrap();
                                    shared_data.from = Some(from);
                                    shared_data.to = Some(to);
                                    shared_data.navmesh = autoroute.navmesh().clone();
                                }
                            }

                            while execute.next(
                                &mut invoker,
                                &mut DebugRouterObserver {
                                    shared_data: shared_data_arc_mutex.clone(),
                                },
                            ) {
                                if let Execute::Autoroute(ref mut autoroute) = execute {
                                    shared_data_arc_mutex.lock().unwrap().navmesh =
                                        autoroute.navmesh().clone();
                                }
                            }
                        });
                    }
                }

                ui.separator();

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

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

                if let Some(layout_arc_mutex) = &self.layout {
                    if let (layout, shared_data, Some(overlay)) = (
                        &layout_arc_mutex.lock().unwrap(),
                        self.shared_data.lock().unwrap(),
                        &mut self.overlay,
                    ) {
                        if ctx.input(|i| i.pointer.any_click()) {
                            overlay.click(
                                layout,
                                point! {x: latest_pos.x as f64, y: -latest_pos.y as f64},
                            );
                        }

                        for primitive in layout.drawing().layer_primitive_nodes(1) {
                            let shape = primitive.primitive(layout.drawing()).shape();

                            let color = if shared_data.highlighteds.contains(&primitive)
                                || overlay
                                    .selection()
                                    .contains(&GenericNode::Primitive(primitive))
                            {
                                egui::Color32::from_rgb(100, 100, 255)
                            } else {
                                egui::Color32::from_rgb(52, 52, 200)
                            };
                            painter.paint_primitive(&shape, color);
                        }

                        for zone in layout.layer_zone_nodes(1) {
                            let color =
                                if overlay.selection().contains(&GenericNode::Compound(zone)) {
                                    egui::Color32::from_rgb(100, 100, 255)
                                } else {
                                    egui::Color32::from_rgb(52, 52, 200)
                                };
                            painter.paint_polygon(&layout.zone(zone).shape().polygon, color)
                        }

                        for primitive in layout.drawing().layer_primitive_nodes(0) {
                            let shape = primitive.primitive(layout.drawing()).shape();

                            let color = if shared_data.highlighteds.contains(&primitive)
                                || overlay
                                    .selection()
                                    .contains(&GenericNode::Primitive(primitive))
                            {
                                egui::Color32::from_rgb(255, 100, 100)
                            } else {
                                egui::Color32::from_rgb(200, 52, 52)
                            };
                            painter.paint_primitive(&shape, color);
                        }

                        for zone in layout.layer_zone_nodes(0) {
                            let color =
                                if overlay.selection().contains(&GenericNode::Compound(zone)) {
                                    egui::Color32::from_rgb(255, 100, 100)
                                } else {
                                    egui::Color32::from_rgb(200, 52, 52)
                                };
                            painter.paint_polygon(&layout.zone(zone).shape().polygon, color)
                        }

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

                        if let Some(navmesh) = &shared_data.navmesh {
                            for edge in navmesh.edge_references() {
                                let from =
                                    edge.source().primitive(layout.drawing()).shape().center();
                                let to = edge.target().primitive(layout.drawing()).shape().center();

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
                                    pos: layout.drawing().primitive(from).shape().center(),
                                    r: 20.0,
                                },
                                egui::Color32::from_rgb(255, 255, 100),
                            );
                            painter.paint_dot(
                                Circle {
                                    pos: layout.drawing().primitive(to).shape().center(),
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
fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    std::thread::spawn(move || futures::executor::block_on(f));
}

#[cfg(target_arch = "wasm32")]
fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

#[cfg(not(target_arch = "wasm32"))]
async fn channel_text(file_handle: rfd::FileHandle) -> String {
    file_handle.path().to_str().unwrap().to_string()
}

#[cfg(target_arch = "wasm32")]
async fn channel_text(file_handle: rfd::FileHandle) -> String {
    std::str::from_utf8(&file_handle.read().await)
        .unwrap()
        .to_string()
}
