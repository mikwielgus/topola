use std::sync::{Arc, Mutex};

use geo::geometry::Point;
use geo::EuclideanDistance;
use petgraph::visit::EdgeRef;
use spade::InsertionError;
use thiserror::Error;

use crate::geometry::primitive::PrimitiveShapeTrait;
use crate::layout::connectivity::BandIndex;
use crate::layout::Layout;
use crate::{
    drawing::{
        dot::FixedDotIndex,
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::MakePrimitiveShape,
        rules::RulesTrait,
    },
    geometry::shape::ShapeTrait,
};

use crate::router::{
    astar::{astar, AstarStrategy, PathTracker},
    draw::DrawException,
    navmesh::{Navmesh, NavmeshEdgeReference, VertexIndex},
    tracer::{Trace, Tracer},
};

#[derive(Error, Debug, Clone, Copy)]
#[error("failed to route from {from:?} to {to:?}")] // this should eventually use Display
pub struct RoutingError {
    from: FixedDotIndex,
    to: FixedDotIndex,
    source: RoutingErrorKind,
}

#[derive(Error, Debug, Clone, Copy)]
pub enum RoutingErrorKind {
    #[error(transparent)]
    MeshInsertion(#[from] InsertionError),
    // exposing more details here seems difficult
    // TODO more descriptive message
    #[error("A* found no path")]
    AStar,
}

pub trait RouterObserverTrait<R: RulesTrait> {
    fn on_rework(&mut self, tracer: &Tracer<R>, trace: &Trace);
    fn before_probe(&mut self, tracer: &Tracer<R>, trace: &Trace, edge: NavmeshEdgeReference);
    fn on_probe(
        &mut self,
        tracer: &Tracer<R>,
        trace: &Trace,
        edge: NavmeshEdgeReference,
        result: Result<(), DrawException>,
    );
    fn on_estimate(&mut self, tracer: &Tracer<R>, vertex: VertexIndex);
}

pub struct Router<R: RulesTrait> {
    layout: Arc<Mutex<Layout<R>>>,
}

struct RouterAstarStrategy<'a, RO: RouterObserverTrait<R>, R: RulesTrait> {
    tracer: Tracer<'a, R>,
    trace: Trace,
    to: FixedDotIndex,
    observer: &'a mut RO,
}

impl<'a, RO: RouterObserverTrait<R>, R: RulesTrait> RouterAstarStrategy<'a, RO, R> {
    pub fn new(
        tracer: Tracer<'a, R>,
        trace: Trace,
        to: FixedDotIndex,
        observer: &'a mut RO,
    ) -> Self {
        Self {
            tracer,
            trace,
            to,
            observer,
        }
    }
}

impl<'a, RO: RouterObserverTrait<R>, R: RulesTrait> AstarStrategy<&Navmesh, f64>
    for RouterAstarStrategy<'a, RO, R>
{
    fn is_goal(&mut self, vertex: VertexIndex, tracker: &PathTracker<&Navmesh>) -> bool {
        let new_path = tracker.reconstruct_path_to(vertex);
        let width = self.trace.width;

        self.tracer
            .rework_path(&mut self.trace, &new_path, width)
            .unwrap();
        self.observer.on_rework(&self.tracer, &self.trace);

        self.tracer.finish(&mut self.trace, self.to, width).is_ok()
    }

    fn edge_cost(&mut self, edge: NavmeshEdgeReference) -> Option<f64> {
        self.observer.before_probe(&self.tracer, &self.trace, edge);
        if edge.target() == self.to.into() {
            return None;
        }

        let before_probe_length = self
            .tracer
            .layout
            .lock()
            .unwrap()
            .band_length(self.trace.band);

        let width = self.trace.width;
        let result = self.tracer.step(&mut self.trace, edge.target(), width);
        self.observer
            .on_probe(&self.tracer, &self.trace, edge, result);

        let probe_length = self
            .tracer
            .layout
            .lock()
            .unwrap()
            .band_length(self.trace.band);

        if result.is_ok() {
            self.tracer.undo_step(&mut self.trace);
            Some(probe_length - before_probe_length)
        } else {
            None
        }
    }

    fn estimate_cost(&mut self, vertex: VertexIndex) -> f64 {
        self.observer.on_estimate(&self.tracer, vertex);

        let layout = self.tracer.layout.lock().unwrap();
        let start_point = PrimitiveIndex::from(vertex)
            .primitive(layout.drawing())
            .shape()
            .center();
        let end_point = layout.drawing().primitive(self.to).shape().center();

        end_point.euclidean_distance(&start_point)
    }
}

impl<R: RulesTrait> Router<R> {
    pub fn new(layout: Arc<Mutex<Layout<R>>>) -> Self {
        Router { layout }
    }

    pub fn route_band(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        width: f64,
        observer: &mut impl RouterObserverTrait<R>,
    ) -> Result<BandIndex, RoutingError> {
        // XXX: Should we actually store the mesh? May be useful for debugging, but doesn't look
        // right.
        //self.mesh.triangulate(&self.layout)?;
        let mesh = Navmesh::new(&self.layout.lock().unwrap()).map_err(|err| RoutingError {
            from,
            to,
            source: err.into(),
        })?;

        let mut tracer = self.tracer(&mesh);

        let trace = tracer.start(from, width);
        let band = trace.band;

        let (_cost, _path) = astar(
            &mesh,
            from.into(),
            &mut RouterAstarStrategy::new(tracer, trace, to.into(), observer),
        )
        .ok_or(RoutingError {
            from,
            to,
            source: RoutingErrorKind::AStar,
        })?;

        Ok(band)
    }

    pub fn reroute_band(
        &mut self,
        band: BandIndex,
        to: Point,
        width: f64,
        observer: &mut impl RouterObserverTrait<R>,
    ) -> Result<BandIndex, RoutingError> {
        let (from_dot, to_dot) = {
            let mut layout = self.layout.lock().unwrap();

            let from_dot = layout.band_from(band);
            let to_dot = layout.band_to(band).unwrap();
            layout.remove_band(band);
            layout.move_dot(to_dot.into(), to).unwrap(); // TODO: Remove `.unwrap()`.
            (from_dot, to_dot)
        };

        self.route_band(from_dot, to_dot, width, observer)
    }

    pub fn tracer<'a>(&'a mut self, mesh: &'a Navmesh) -> Tracer<R> {
        Tracer::new(self.layout.clone(), mesh)
    }

    pub fn layout(&self) -> Arc<Mutex<Layout<R>>> {
        self.layout.clone()
    }
}
