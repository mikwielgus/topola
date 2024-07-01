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
        trace::Trace,
        tracer::Tracer,
        Router, RouterAstarStrategy, RouterError, RouterStatus,
    },
};

#[derive(Debug)]
pub struct Route {
    astar: Astar<Navmesh, f64>,
    trace: Trace,
}

impl Route {
    pub fn new(
        router: &mut Router<impl RulesTrait>,
        from: FixedDotIndex,
        to: FixedDotIndex,
        width: f64,
    ) -> Result<Self, RouterError> {
        let navmesh = Navmesh::new(router.layout(), from, to)?;
        Ok(Self::new_from_navmesh(router, navmesh, width))
    }

    pub fn new_from_navmesh(
        router: &mut Router<impl RulesTrait>,
        navmesh: Navmesh,
        width: f64,
    ) -> Self {
        let source = navmesh.origin();
        let source_navvertex = navmesh.origin_navvertex();
        let target = navmesh.destination();

        let mut tracer = Tracer::new(router.layout_mut());
        let mut trace = tracer.start(source, source_navvertex, width);

        let mut strategy = RouterAstarStrategy::new(tracer, &mut trace, target);
        let astar = Astar::new(navmesh, source_navvertex, &mut strategy);

        Self { astar, trace }
    }

    pub fn step(
        &mut self,
        router: &mut Router<impl RulesTrait>,
    ) -> Result<RouterStatus, RouterError> {
        let tracer = Tracer::new(router.layout_mut());
        let target = self.astar.graph.destination();
        let mut strategy = RouterAstarStrategy::new(tracer, &mut self.trace, target);

        match self.astar.step(&mut strategy)? {
            AstarStatus::Running => Ok(RouterStatus::Running),
            AstarStatus::Finished(_cost, _path, band) => Ok(RouterStatus::Finished(band)),
        }
    }

    pub fn navmesh(&self) -> &Navmesh {
        &self.astar.graph
    }

    pub fn trace(&self) -> &Trace {
        &self.trace
    }
}
