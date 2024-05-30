use std::sync::{Arc, Mutex};

use geo::geometry::Point;
use geo::EuclideanDistance;
use petgraph::visit::EdgeRef;
use spade::InsertionError;
use thiserror::Error;

use crate::{
    drawing::{
        band::BandIndex,
        dot::FixedDotIndex,
        graph::{MakePrimitive, PrimitiveIndex},
        guide::HeadTrait,
        primitive::MakePrimitiveShape,
        rules::RulesTrait,
    },
    geometry::shape::ShapeTrait,
    layout::Layout,
    router::{
        astar::{astar, AstarError, AstarStrategy, PathTracker},
        draw::DrawException,
        navmesh::{Navmesh, NavmeshEdgeReference, NavmeshError, VertexIndex},
        tracer::{Trace, Tracer},
    },
};

#[derive(Error, Debug, Clone)]
#[error("routing failed")]
pub enum RouterError {
    Navmesh(#[from] NavmeshError),
    Astar(#[from] AstarError),
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

pub struct EmptyRouterObserver;

impl<R: RulesTrait> RouterObserverTrait<R> for EmptyRouterObserver {
    fn on_rework(&mut self, _tracer: &Tracer<R>, _trace: &Trace) {}
    fn before_probe(&mut self, _tracer: &Tracer<R>, _trace: &Trace, _edge: NavmeshEdgeReference) {}
    fn on_probe(
        &mut self,
        _tracer: &Tracer<R>,
        _trace: &Trace,
        _edge: NavmeshEdgeReference,
        _result: Result<(), DrawException>,
    ) {
    }
    fn on_estimate(&mut self, _tracer: &Tracer<R>, _vertex: VertexIndex) {}
}

pub struct Router<'a, R: RulesTrait> {
    layout: &'a mut Layout<R>,
    navmesh: Navmesh,
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

impl<'a, RO: RouterObserverTrait<R>, R: RulesTrait> AstarStrategy<&Navmesh, f64, BandIndex>
    for RouterAstarStrategy<'a, RO, R>
{
    fn is_goal(
        &mut self,
        vertex: VertexIndex,
        tracker: &PathTracker<&Navmesh>,
    ) -> Option<BandIndex> {
        let new_path = tracker.reconstruct_path_to(vertex);
        let width = self.trace.width;

        self.tracer
            .rework_path(&mut self.trace, &new_path, width)
            .unwrap();
        self.observer.on_rework(&self.tracer, &self.trace);

        self.tracer.finish(&mut self.trace, self.to, width).ok()
    }

    fn edge_cost(&mut self, edge: NavmeshEdgeReference) -> Option<f64> {
        self.observer.before_probe(&self.tracer, &self.trace, edge);
        if edge.target() == self.to.into() {
            return None;
        }

        let before_probe_length = self.tracer.layout.band_length(self.trace.head.face());

        let width = self.trace.width;
        let result = self.tracer.step(&mut self.trace, edge.target(), width);
        self.observer
            .on_probe(&self.tracer, &self.trace, edge, result);

        let probe_length = self.tracer.layout.band_length(self.trace.head.face());

        if result.is_ok() {
            self.tracer.undo_step(&mut self.trace);
            Some(probe_length - before_probe_length)
        } else {
            None
        }
    }

    fn estimate_cost(&mut self, vertex: VertexIndex) -> f64 {
        self.observer.on_estimate(&self.tracer, vertex);

        let start_point = PrimitiveIndex::from(vertex)
            .primitive(self.tracer.layout.drawing())
            .shape()
            .center();
        let end_point = self
            .tracer
            .layout
            .drawing()
            .primitive(self.to)
            .shape()
            .center();

        end_point.euclidean_distance(&start_point)
    }
}

impl<'a, R: RulesTrait> Router<'a, R> {
    pub fn new(
        layout: &'a mut Layout<R>,
        from: FixedDotIndex,
        to: FixedDotIndex,
    ) -> Result<Self, RouterError> {
        let navmesh = { Navmesh::new(layout, from, to)? };
        Ok(Self::new_from_navmesh(layout, navmesh))
    }

    pub fn new_from_navmesh(layout: &'a mut Layout<R>, navmesh: Navmesh) -> Self {
        Self { layout, navmesh }
    }

    pub fn route_band(
        &mut self,
        width: f64,
        observer: &mut impl RouterObserverTrait<R>,
    ) -> Result<BandIndex, RouterError> {
        let from = self.navmesh.from();
        let to = self.navmesh.to();
        let mut tracer = Tracer::new(self.layout);
        let trace = tracer.start(from, width);

        let (_cost, _path, band) = astar(
            &self.navmesh,
            from.into(),
            &mut RouterAstarStrategy::new(tracer, trace, to, observer),
        )?;

        Ok(band)
    }

    /*pub fn reroute_band(
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
    }*/

    pub fn layout(&mut self) -> &mut Layout<R> {
        self.layout
    }
}
