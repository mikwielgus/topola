use geo::EuclideanDistance;
use petgraph::{data::DataMap, visit::EdgeRef};
use thiserror::Error;

use crate::{
    drawing::{
        band::BandFirstSegIndex,
        dot::{DotIndex, FixedDotIndex},
        graph::{MakePrimitive, PrimitiveIndex},
        head::{GetFace, Head},
        primitive::MakePrimitiveShape,
        rules::AccessRules,
    },
    geometry::{
        primitive::AccessPrimitiveShape,
        shape::{AccessShape, MeasureLength},
    },
    graph::GetPetgraphIndex,
    layout::Layout,
    router::{
        astar::{AstarError, AstarStatus, AstarStrategy, PathTracker},
        navmesh::{Navmesh, NavmeshEdgeReference, NavmeshError, NavvertexIndex},
        route::Route,
        trace::Trace,
        tracer::Tracer,
    },
};

#[derive(Error, Debug, Clone)]
#[error("routing failed")]
pub enum RouterError {
    Navmesh(#[from] NavmeshError),
    Astar(#[from] AstarError),
}

#[derive(Debug)]
pub enum RouterStatus {
    Running,
    Finished(BandFirstSegIndex),
}

#[derive(Debug)]
pub struct RouterAstarStrategy<'a, R: AccessRules> {
    pub tracer: Tracer<'a, R>,
    pub trace: &'a mut Trace,
    pub target: FixedDotIndex,
}

impl<'a, R: AccessRules> RouterAstarStrategy<'a, R> {
    pub fn new(tracer: Tracer<'a, R>, trace: &'a mut Trace, target: FixedDotIndex) -> Self {
        Self {
            tracer,
            trace,
            target,
        }
    }

    fn bihead_length(&self) -> f64 {
        self.trace.head.ref_(self.tracer.layout.drawing()).length()
            + match self.trace.head.face() {
                DotIndex::Fixed(..) => 0.0,
                DotIndex::Loose(face) => self
                    .tracer
                    .layout
                    .drawing()
                    .guide()
                    .rear_head(face)
                    .ref_(self.tracer.layout.drawing())
                    .length(),
            }
    }
}

impl<'a, R: AccessRules> AstarStrategy<Navmesh, f64, (), (), BandFirstSegIndex>
    for RouterAstarStrategy<'a, R>
{
    fn is_goal(
        &mut self,
        navmesh: &Navmesh,
        vertex: NavvertexIndex,
        tracker: &PathTracker<Navmesh>,
    ) -> Option<BandFirstSegIndex> {
        let new_path = tracker.reconstruct_path_to(vertex);
        let width = self.trace.width;

        self.tracer
            .rework_path(navmesh, &mut self.trace, &new_path[..], width)
            .unwrap();

        self.tracer
            .finish(navmesh, &mut self.trace, self.target, width)
            .ok()
    }

    fn probe(&mut self, navmesh: &Navmesh, edge: NavmeshEdgeReference) -> Result<(f64, ()), ()> {
        if edge.target().petgraph_index() == self.target.petgraph_index() {
            return Err(());
        }

        let prev_bihead_length = self.bihead_length();

        let width = self.trace.width;
        let result = self
            .trace
            .step(&mut self.tracer, navmesh, edge.target(), width);

        let probe_length = self.bihead_length() - prev_bihead_length;

        if result.is_ok() {
            self.trace.undo_step(&mut self.tracer);
            Ok((probe_length, ()))
        } else {
            Err(())
        }
    }

    fn estimate_cost(&mut self, navmesh: &Navmesh, vertex: NavvertexIndex) -> f64 {
        let start_point = PrimitiveIndex::from(navmesh.node_weight(vertex).unwrap().node)
            .primitive(self.tracer.layout.drawing())
            .shape()
            .center();
        let end_point = self
            .tracer
            .layout
            .drawing()
            .primitive(self.target)
            .shape()
            .center();

        end_point.euclidean_distance(&start_point)
    }
}

#[derive(Debug)]
pub struct Router<'a, R: AccessRules> {
    layout: &'a mut Layout<R>,
}

impl<'a, R: AccessRules> Router<'a, R> {
    pub fn new(layout: &'a mut Layout<R>) -> Self {
        Self { layout }
    }

    pub fn route(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        width: f64,
    ) -> Result<BandFirstSegIndex, RouterError> {
        let mut route = self.route_walk(from, to, width)?;

        loop {
            let status = match route.step(self) {
                Ok(status) => status,
                Err(err) => return Err(err),
            };

            if let RouterStatus::Finished(band) = status {
                return Ok(band);
            }
        }
    }

    pub fn route_walk(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        width: f64,
    ) -> Result<Route, RouterError> {
        Route::new(self, from, to, width)
    }

    pub fn layout_mut(&mut self) -> &mut Layout<R> {
        &mut self.layout
    }

    pub fn layout(&self) -> &Layout<R> {
        &self.layout
    }
}
