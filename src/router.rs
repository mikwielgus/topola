use geo::geometry::Point;
use petgraph::visit::EdgeRef;
use spade::InsertionError;
use thiserror::Error;

use crate::astar::{astar, AstarStrategy, PathTracker};
use crate::connectivity::BandIndex;
use crate::draw::DrawException;
use crate::geometry::FixedDotIndex;
use crate::guide::HeadTrait;
use crate::layout::Layout;

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

pub trait RouterObserver {
    fn on_rework(&mut self, tracer: &Tracer, trace: &Trace);
    fn before_probe(&mut self, tracer: &Tracer, trace: &Trace, edge: MeshEdgeReference);
    fn on_probe(
        &mut self,
        tracer: &Tracer,
        trace: &Trace,
        edge: MeshEdgeReference,
        result: Result<(), DrawException>,
    );
    fn on_estimate(&mut self, tracer: &Tracer, vertex: VertexIndex);
}

pub struct Router {
    pub layout: Layout,
    rules: Rules,
}

struct RouterAstarStrategy<'a, RO: RouterObserver> {
    tracer: Tracer<'a>,
    trace: Trace,
    to: FixedDotIndex,
    observer: &'a mut RO,
}

impl<'a, RO: RouterObserver> RouterAstarStrategy<'a, RO> {
    pub fn new(tracer: Tracer<'a>, trace: Trace, to: FixedDotIndex, observer: &'a mut RO) -> Self {
        Self {
            tracer,
            trace,
            to,
            observer,
        }
    }
}

impl<'a, RO: RouterObserver> AstarStrategy<&Mesh, u64> for RouterAstarStrategy<'a, RO> {
    fn is_goal(&mut self, vertex: VertexIndex, tracker: &PathTracker<&Mesh>) -> bool {
        let new_path = tracker.reconstruct_path_to(vertex);

        self.tracer
            .rework_path(&mut self.trace, &new_path, 5.0)
            .unwrap();
        self.observer.on_rework(&self.tracer, &self.trace);

        self.tracer.finish(&mut self.trace, self.to, 5.0).is_ok()
    }

    fn edge_cost(&mut self, edge: MeshEdgeReference) -> Option<u64> {
        self.observer.before_probe(&self.tracer, &self.trace, edge);
        if edge.target() == self.to.into() {
            return None;
        }

        let result = self.tracer.step(&mut self.trace, edge.target(), 5.0);
        self.observer
            .on_probe(&self.tracer, &self.trace, edge, result);

        if result.is_ok() {
            self.tracer.undo_step(&mut self.trace);
            Some(1)
        } else {
            None
        }
    }

    fn estimate_cost(&mut self, vertex: VertexIndex) -> u64 {
        self.observer.on_estimate(&self.tracer, vertex);
        0
    }
}

impl Router {
    pub fn new() -> Self {
        Router {
            layout: Layout::new(),
            rules: Rules::new(),
        }
    }

    pub fn route_band(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        observer: &mut impl RouterObserver,
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

    pub fn reroute(
        &mut self,
        _from: FixedDotIndex,
        _to: Point,
        _observer: &mut impl RouterObserver,
    ) -> Result<Mesh, InsertionError> {
        Ok(Mesh::new(&self.layout))
    }

    pub fn tracer<'a>(&'a mut self, mesh: &'a Mesh) -> Tracer {
        Tracer::new(&mut self.layout, &self.rules, mesh)
    }
}
