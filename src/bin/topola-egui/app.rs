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
        invoker::{Command, Execute, ExecuteWithStatus, Invoker, InvokerStatus},
        Autorouter,
    },
    drawing::{
        dot::FixedDotIndex,
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::MakePrimitiveShape,
        rules::AccessRules,
        Drawing, Infringement, LayoutException,
    },
    geometry::{
        compound::ManageCompounds,
        primitive::{AccessPrimitiveShape, BendShape, DotShape, PrimitiveShape, SegShape},
        shape::AccessShape,
        GenericNode,
    },
    layout::{via::ViaWeight, zone::MakePolyShape, Layout},
    math::Circle,
    router::{
        draw::DrawException,
        navmesh::{BinavvertexNodeIndex, Navmesh},
        trace::Trace,
        tracer::Tracer,
    },
    specctra::{design::SpecctraDesign, mesadata::SpecctraMesadata},
};

use crate::{
    bottom::Bottom, file_receiver::FileReceiver, layers::Layers, overlay::Overlay,
    painter::Painter, top::Top, viewport::Viewport,
};

/// Deserialize/Serialize is needed to persist app state between restarts.
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct App {
    #[serde(skip)]
    maybe_overlay: Option<Overlay>,

    #[serde(skip)]
    arc_mutex_maybe_invoker: Arc<Mutex<Option<Invoker<SpecctraMesadata>>>>,

    #[serde(skip)]
    maybe_execute: Option<ExecuteWithStatus>,

    #[serde(skip)]
    content_channel: (Sender<String>, Receiver<String>),

    #[serde(skip)]
    viewport: Viewport,

    #[serde(skip)]
    top: Top,

    #[serde(skip)]
    bottom: Bottom,

    #[serde(skip)]
    maybe_layers: Option<Layers>,

    #[serde(skip)]
    update_counter: f32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            maybe_overlay: None,
            arc_mutex_maybe_invoker: Arc::new(Mutex::new(None)),
            maybe_execute: None,
            content_channel: channel(),
            viewport: Viewport::new(),
            top: Top::new(),
            bottom: Bottom::new(),
            maybe_layers: None,
            update_counter: 0.0,
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

    fn update_state(&mut self, dt: f32) {
        self.update_counter += dt;

        if self.update_counter <= 0.1 {
            return;
        }

        self.update_counter = 0.0;

        let mut file_receiver = FileReceiver::new(&self.content_channel.1);

        if let Ok(bufread) = file_receiver.try_recv() {
            let design = SpecctraDesign::load(bufread).unwrap();
            let board = design.make_board();
            self.maybe_overlay = Some(Overlay::new(&board).unwrap());
            self.maybe_layers = Some(Layers::new(&board));
            self.arc_mutex_maybe_invoker = Arc::new(Mutex::new(Some(Invoker::new(
                Autorouter::new(board).unwrap(),
            ))))
        }

        if let Some(invoker) = self.arc_mutex_maybe_invoker.lock().unwrap().as_mut() {
            if let Some(ref mut execute) = self.maybe_execute {
                let status = match execute.step(invoker) {
                    Ok(status) => status,
                    Err(err) => return,
                };
            }
        }
    }
}

impl eframe::App for App {
    /// Called to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI has to be repainted.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_state(ctx.input(|i| i.stable_dt));

        self.top.update(
            ctx,
            self.content_channel.0.clone(),
            self.arc_mutex_maybe_invoker.clone(),
            &mut self.maybe_execute,
            &mut self.maybe_overlay,
        );

        if let Some(ref mut layers) = self.maybe_layers {
            if let Some(invoker) = self.arc_mutex_maybe_invoker.lock().unwrap().as_ref() {
                layers.update(ctx, invoker.autorouter().board());
            }
        }

        let viewport_rect = self.viewport.update(
            ctx,
            &self.top,
            &mut self.arc_mutex_maybe_invoker.lock().unwrap(),
            &mut self.maybe_execute,
            &mut self.maybe_overlay,
            &self.maybe_layers,
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
