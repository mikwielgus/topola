use geo::EuclideanDistance;
use petgraph::{
    data::DataMap,
    graph::{EdgeReference, NodeIndex, UnGraph},
    visit::EdgeRef,
};
use thiserror::Error;

use crate::{
    drawing::{
        band::BandFirstSegIndex,
        dot::{DotIndex, FixedDotIndex},
        graph::{MakePrimitive, PrimitiveIndex},
        guide::{CaneHead, Head, HeadTrait},
        primitive::MakePrimitiveShape,
        rules::RulesTrait,
    },
    geometry::{primitive::PrimitiveShapeTrait, shape::ShapeTrait},
    graph::GetPetgraphIndex,
    layout::Layout,
    router::{
        astar::{astar, Astar, AstarError, AstarStatus, AstarStrategy, PathTracker},
        draw::DrawException,
        navmesh::{
            BinavvertexNodeIndex, Navmesh, NavmeshEdgeReference, NavmeshError, NavvertexIndex,
            NavvertexWeight,
        },
        tracer::{Trace, Tracer},
    },
};

#[derive(Error, Debug, Clone)]
#[error("routing failed")]
pub enum RouterError {
    Navmesh(#[from] NavmeshError),
    Astar(#[from] AstarError),
}

pub struct Router {
    astar: Astar<Navmesh, f64>,
    trace: Trace,
}

struct RouterAstarStrategy<'a, R: RulesTrait> {
    pub tracer: Tracer<'a, R>,
    pub trace: &'a mut Trace,
    pub target: FixedDotIndex,
}

impl<'a, R: RulesTrait> RouterAstarStrategy<'a, R> {
    pub fn new(tracer: Tracer<'a, R>, trace: &'a mut Trace, target: FixedDotIndex) -> Self {
        Self {
            tracer,
            trace,
            target,
        }
    }

    fn bihead_length(&self) -> f64 {
        self.head_length(&self.trace.head)
            + match self.trace.head.face() {
                DotIndex::Fixed(..) => 0.0,
                DotIndex::Loose(face) => {
                    self.head_length(&self.tracer.layout.drawing().guide().rear_head(face))
                }
            }
    }

    fn head_length(&self, head: &Head) -> f64 {
        match head {
            Head::Bare(..) => 0.0,
            Head::Cane(cane_head) => {
                self.tracer
                    .layout
                    .drawing()
                    .primitive(cane_head.cane.seg)
                    .shape()
                    .length()
                    + self
                        .tracer
                        .layout
                        .drawing()
                        .primitive(cane_head.cane.bend)
                        .shape()
                        .length()
            }
        }
    }
}

impl<'a, R: RulesTrait> AstarStrategy<Navmesh, f64, BandFirstSegIndex>
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

    fn edge_cost(&mut self, navmesh: &Navmesh, edge: NavmeshEdgeReference) -> Option<f64> {
        if edge.target().petgraph_index() == self.target.petgraph_index() {
            return None;
        }

        let prev_bihead_length = self.bihead_length();

        let width = self.trace.width;
        let result = self
            .tracer
            .step(navmesh, &mut self.trace, edge.target(), width);

        let probe_length = self.bihead_length() - prev_bihead_length;

        if result.is_ok() {
            self.tracer.undo_step(navmesh, &mut self.trace);
            Some(probe_length)
        } else {
            None
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

impl Router {
    pub fn new(
        layout: &mut Layout<impl RulesTrait>,
        from: FixedDotIndex,
        to: FixedDotIndex,
        width: f64,
    ) -> Result<Self, RouterError> {
        let navmesh = Navmesh::new(layout, from, to)?;
        Ok(Self::new_from_navmesh(layout, navmesh, width))
    }

    pub fn new_from_navmesh(
        layout: &mut Layout<impl RulesTrait>,
        navmesh: Navmesh,
        width: f64,
    ) -> Self {
        let source = navmesh.source();
        let source_navvertex = navmesh.source_navvertex();
        let target = navmesh.target();

        let mut tracer = Tracer::new(layout);
        let mut trace = tracer.start(&navmesh, source, source_navvertex, width);

        let mut strategy = RouterAstarStrategy::new(tracer, &mut trace, target);
        let astar = Astar::new(navmesh, source_navvertex, &mut strategy);

        Self { astar, trace }
    }

    pub fn route_band(
        &mut self,
        layout: &mut Layout<impl RulesTrait>,
        _width: f64,
    ) -> Result<BandFirstSegIndex, RouterError> {
        let tracer = Tracer::new(layout);
        let target = self.astar.graph.target();
        let mut strategy = RouterAstarStrategy::new(tracer, &mut self.trace, target);

        loop {
            let status = match self.astar.step(&mut strategy) {
                Ok(status) => status,
                Err(err) => return Err(err.into()),
            };

            if let AstarStatus::Finished(_cost, _path, band) = status {
                return Ok(band);
            }
        }
    }
}
