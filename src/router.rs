use geo::geometry::Point;
use geo::EuclideanDistance;
use petgraph::visit::EdgeRef;
use spade::InsertionError;
use thiserror::Error;

use crate::astar::{astar, AstarStrategy, PathTracker};
use crate::draw::DrawException;
use crate::layout::guide::HeadTrait;
use crate::layout::rules::RulesTrait;
use crate::layout::Layout;
use crate::layout::{
    connectivity::BandIndex,
    dot::FixedDotIndex,
    geometry::shape::ShapeTrait,
    graph::{GeometryIndex, MakePrimitive},
    primitive::MakeShape,
};

use crate::mesh::{Mesh, MeshEdgeReference, VertexIndex};

use crate::rules::Rules;
use crate::tracer::{Trace, Tracer};

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
    fn before_probe(&mut self, tracer: &Tracer<R>, trace: &Trace, edge: MeshEdgeReference);
    fn on_probe(
        &mut self,
        tracer: &Tracer<R>,
        trace: &Trace,
        edge: MeshEdgeReference,
        result: Result<(), DrawException>,
    );
    fn on_estimate(&mut self, tracer: &Tracer<R>, vertex: VertexIndex);
}

pub struct Router<R: RulesTrait> {
    pub layout: Layout<R>,
    rules: Rules,
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

impl<'a, RO: RouterObserverTrait<R>, R: RulesTrait> AstarStrategy<&Mesh, f64>
    for RouterAstarStrategy<'a, RO, R>
{
    fn is_goal(&mut self, vertex: VertexIndex, tracker: &PathTracker<&Mesh>) -> bool {
        let new_path = tracker.reconstruct_path_to(vertex);

        self.tracer
            .rework_path(&mut self.trace, &new_path, 5.0)
            .unwrap();
        self.observer.on_rework(&self.tracer, &self.trace);

        self.tracer.finish(&mut self.trace, self.to, 5.0).is_ok()
    }

    fn edge_cost(&mut self, edge: MeshEdgeReference) -> Option<f64> {
        self.observer.before_probe(&self.tracer, &self.trace, edge);
        if edge.target() == self.to.into() {
            return None;
        }

        let before_probe_length = self.tracer.layout.band(self.trace.head.band()).length();

        let result = self.tracer.step(&mut self.trace, edge.target(), 5.0);
        self.observer
            .on_probe(&self.tracer, &self.trace, edge, result);

        let probe_length = self.tracer.layout.band(self.trace.head.band()).length();

        if result.is_ok() {
            self.tracer.undo_step(&mut self.trace);
            Some(probe_length - before_probe_length)
        } else {
            None
        }
    }

    fn estimate_cost(&mut self, vertex: VertexIndex) -> f64 {
        self.observer.on_estimate(&self.tracer, vertex);
        let start_point = GeometryIndex::from(vertex)
            .primitive(self.tracer.layout)
            .shape()
            .center();
        let end_point = self.tracer.layout.primitive(self.to).shape().center();
        end_point.euclidean_distance(&start_point)
    }
}

impl<R: RulesTrait> Router<R> {
    pub fn new(rules: R) -> Self {
        Router {
            layout: Layout::new(rules),
            rules: Rules::new(),
        }
    }

    pub fn route_band(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        observer: &mut impl RouterObserverTrait<R>,
    ) -> Result<BandIndex, RoutingError> {
        // XXX: Should we actually store the mesh? May be useful for debugging, but doesn't look
        // right.
        //self.mesh.triangulate(&self.layout)?;
        let mut mesh = Mesh::new(&self.layout);
        mesh.generate(&self.layout).map_err(|err| RoutingError {
            from,
            to,
            source: err.into(),
        })?;

        let mut tracer = self.tracer(&mesh);
        let trace = tracer.start(from, 3.0);
        let band = trace.head.band();

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
        observer: &mut impl RouterObserverTrait<R>,
    ) -> Result<BandIndex, RoutingError> {
        let from_dot = self.layout.band(band).from();
        let to_dot = self.layout.band(band).to().unwrap();
        self.layout.remove_band(band);
        self.layout.move_dot(to_dot.into(), to).unwrap(); // TODO: Remove `.unwrap()`.
        self.route_band(from_dot, to_dot, observer)
    }

    pub fn tracer<'a>(&'a mut self, mesh: &'a Mesh) -> Tracer<R> {
        Tracer::new(&mut self.layout, &self.rules, mesh)
    }
}
