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
    specctra::{design::SpecctraDesign, mesadata::SpecctraMesadata},
};

use crate::{
    bottom::Bottom, layers::Layers, overlay::Overlay, painter::Painter, top::Top,
    viewport::Viewport,
};

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
    invoker: Option<Arc<Mutex<Invoker<SpecctraMesadata>>>>,

    #[serde(skip)]
    shared_data: Arc<Mutex<SharedData>>,

    #[serde(skip)]
    text_channel: (Sender<String>, Receiver<String>),

    #[serde(skip)]
    viewport: Viewport,

    #[serde(skip)]
    top: Top,

    #[serde(skip)]
    bottom: Bottom,

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
            viewport: Viewport::new(),
            top: Top::new(),
            bottom: Bottom::new(),
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
                let design = SpecctraDesign::load_from_string(file_contents).unwrap();
                let board = design.make_board();
                self.overlay = Some(Overlay::new(&board).unwrap());
                self.layers = Some(Layers::new(&board));
                self.invoker = Some(Arc::new(Mutex::new(Invoker::new(
                    Autorouter::new(board).unwrap(),
                ))));
            }
        } else {
            if let Ok(path) = self.text_channel.1.try_recv() {
                let design = SpecctraDesign::load_from_file(&path).unwrap();
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
            &mut self.overlay,
        );

        if let Some(ref mut layers) = self.layers {
            if let Some(invoker_arc_mutex) = &self.invoker {
                layers.update(ctx, invoker_arc_mutex.lock().unwrap().autorouter().board());
            }
        }

        let viewport_rect = self.viewport.update(
            ctx,
            &self.top,
            self.shared_data.clone(),
            &self.invoker,
            &mut self.overlay,
            &self.layers,
        );

        self.bottom.update(ctx, &self.viewport, viewport_rect);

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
