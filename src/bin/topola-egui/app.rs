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
use unic_langid::{langid, LanguageIdentifier};

use topola::{
    autorouter::{invoker::Invoker, Autorouter},
    drawing::{
        dot::FixedDotIndex,
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::MakePrimitiveShape,
        rules::AccessRules,
        Drawing, DrawingException, Infringement,
    },
    geometry::{
        compound::ManageCompounds,
        primitive::{AccessPrimitiveShape, BendShape, DotShape, PrimitiveShape, SegShape},
        shape::AccessShape,
        GenericNode,
    },
    layout::{poly::MakePolyShape, via::ViaWeight, Layout},
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
    activity::{ActivityStatus, ActivityWithStatus},
    bottom::Bottom,
    error_dialog::ErrorDialog,
    file_receiver::FileReceiver,
    layers::Layers,
    overlay::Overlay,
    painter::Painter,
    top::Top,
    translator::Translator,
    viewport::Viewport,
};

/// Deserialize/Serialize is needed to persist app state between restarts.
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct App {
    translator: Translator,

    #[serde(skip)]
    maybe_overlay: Option<Overlay>,

    #[serde(skip)]
    arc_mutex_maybe_invoker: Arc<Mutex<Option<Invoker<SpecctraMesadata>>>>,

    #[serde(skip)]
    maybe_activity: Option<ActivityWithStatus>,

    #[serde(skip)]
    content_channel: (Sender<String>, Receiver<String>),

    #[serde(skip)]
    history_channel: (Sender<String>, Receiver<String>),

    #[serde(skip)]
    viewport: Viewport,

    #[serde(skip)]
    top: Top,

    #[serde(skip)]
    bottom: Bottom,

    #[serde(skip)]
    error_dialog: ErrorDialog,

    #[serde(skip)]
    maybe_layers: Option<Layers>,

    #[serde(skip)]
    maybe_design: Option<SpecctraDesign>,

    #[serde(skip)]
    update_counter: f32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            translator: Translator::new(langid!("en-US")),
            maybe_overlay: None,
            arc_mutex_maybe_invoker: Arc::new(Mutex::new(None)),
            maybe_activity: None,
            content_channel: channel(),
            history_channel: channel(),
            viewport: Viewport::new(),
            top: Top::new(),
            bottom: Bottom::new(),
            error_dialog: ErrorDialog::new(),
            maybe_layers: None,
            maybe_design: None,
            update_counter: 0.0,
        }
    }
}

impl App {
    /// Called once on start.
    pub fn new(cc: &eframe::CreationContext<'_>, langid: LanguageIdentifier) -> Self {
        // Load previous app state if one exists.
        if let Some(storage) = cc.storage {
            let this = Self {
                translator: Translator::new(langid),
                ..eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
            };
            return this;
        }

        Self {
            translator: Translator::new(langid),
            ..Default::default()
        }
    }

    fn advance_state_by_dt(&mut self, dt: f32) {
        self.update_counter += dt;

        while self.update_counter >= self.top.frame_timestep {
            self.update_counter -= self.top.frame_timestep;

            if !self.update_state() {
                return;
            }
        }
    }

    fn update_state(&mut self) -> bool {
        let mut content_file_receiver = FileReceiver::new(&self.content_channel.1);

        if let Ok(bufread) = content_file_receiver.try_recv() {
            let design = SpecctraDesign::load(bufread).unwrap();
            let board = design.make_board();
            self.maybe_overlay = Some(Overlay::new(&board).unwrap());
            self.maybe_layers = Some(Layers::new(&board));
            self.maybe_design = Some(design);
            self.arc_mutex_maybe_invoker = Arc::new(Mutex::new(Some(Invoker::new(
                Autorouter::new(board).unwrap(),
            ))));
            self.viewport.scheduled_zoom_to_fit = true;
        }

        if let Some(invoker) = self.arc_mutex_maybe_invoker.lock().unwrap().as_mut() {
            let mut history_file_receiver = FileReceiver::new(&self.history_channel.1);

            if let Ok(bufread) = history_file_receiver.try_recv() {
                invoker.replay(serde_json::from_reader(bufread).unwrap())
            }

            if let Some(ref mut activity) = self.maybe_activity {
                match activity.step(invoker) {
                    Ok(ActivityStatus::Running) => return true,
                    Ok(ActivityStatus::Finished(..)) => return false,
                    Err(err) => return false,
                }
            }
        }

        false
    }
}

impl eframe::App for App {
    /// Called to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI has to be repainted.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.top.update(
            ctx,
            &self.translator,
            self.content_channel.0.clone(),
            self.history_channel.0.clone(),
            self.arc_mutex_maybe_invoker.clone(),
            &mut self.maybe_activity,
            &mut self.viewport,
            &mut self.maybe_overlay,
            &self.maybe_design,
        );

        self.advance_state_by_dt(ctx.input(|i| i.stable_dt));

        self.bottom
            .update(ctx, &self.translator, &self.viewport, &self.maybe_activity);

        if self.top.show_layer_manager {
            if let Some(ref mut layers) = self.maybe_layers {
                if let Some(invoker) = self.arc_mutex_maybe_invoker.lock().unwrap().as_ref() {
                    layers.update(ctx, invoker.autorouter().board());
                }
            }
        }

        self.error_dialog
            .update(ctx, &self.translator, &self.viewport);

        let viewport_rect = self.viewport.update(
            ctx,
            &self.top,
            &mut self.arc_mutex_maybe_invoker.lock().unwrap(),
            &mut self.maybe_activity,
            &mut self.maybe_overlay,
            &self.maybe_layers,
        );

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    std::thread::spawn(move || futures_lite::future::block_on(f));
}

#[cfg(target_arch = "wasm32")]
pub fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}
