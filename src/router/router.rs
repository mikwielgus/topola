use std::sync::{Arc, Mutex};

use geo::geometry::Point;
use geo::EuclideanDistance;
use petgraph::visit::EdgeRef;
use spade::InsertionError;
use thiserror::Error;

use crate::drawing::graph::{GetLayer, GetMaybeNet};
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

pub struct Router<'a, R: RulesTrait> {
    layout: &'a mut Arc<Mutex<Layout<R>>>,
    navmesh: Navmesh,
}

struct RouterAstarStrategy<'a, RO: RouterObserverTrait<R>, R: RulesTrait> {
    tracer: Tracer<R>,
    trace: Trace,
    to: FixedDotIndex,
    observer: &'a mut RO,
}

impl<'a, RO: RouterObserverTrait<R>, R: RulesTrait> RouterAstarStrategy<'a, RO, R> {
    pub fn new(tracer: Tracer<R>, trace: Trace, to: FixedDotIndex, observer: &'a mut RO) -> Self {
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

impl<'a, R: RulesTrait> Router<'a, R> {
    pub fn new(
        layout: &'a mut Arc<Mutex<Layout<R>>>,
        from: FixedDotIndex,
        to: FixedDotIndex,
    ) -> Result<Self, InsertionError> {
        let navmesh = {
            let layout = layout.lock().unwrap();
            Navmesh::new(&layout, from, to)?
        };
        Self::new_from_navmesh(layout, navmesh)
    }

    pub fn new_from_navmesh(
        layout: &'a mut Arc<Mutex<Layout<R>>>,
        navmesh: Navmesh,
    ) -> Result<Self, InsertionError> {
        Ok(Self { layout, navmesh })
    }

    pub fn route_band(
        &mut self,
        width: f64,
        observer: &mut impl RouterObserverTrait<R>,
    ) -> Result<BandIndex, RoutingError> {
        let mut tracer = self.tracer();

        let trace = tracer.start(self.navmesh.from(), width);
        let band = trace.band;

        let (_cost, _path) = astar(
            &self.navmesh,
            self.navmesh.from().into(),
            &mut RouterAstarStrategy::new(tracer, trace, self.navmesh.to(), observer),
        )
        .ok_or(RoutingError {
            from: self.navmesh.from(),
            to: self.navmesh.to(),
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
        {
            let mut layout = self.layout.lock().unwrap();

            layout.remove_band(band);
            layout.move_dot(self.navmesh.to().into(), to).unwrap(); // TODO: Remove `.unwrap()`.
        }

        self.route_band(width, observer)
    }

    fn tracer(&mut self) -> Tracer<R> {
        Tracer::new(self.layout.clone())
    }

    pub fn layout(&self) -> Arc<Mutex<Layout<R>>> {
        self.layout.clone()
    }
}
