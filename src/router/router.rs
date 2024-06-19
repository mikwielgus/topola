use geo::EuclideanDistance;
use petgraph::{
    graph::{EdgeReference, NodeIndex, UnGraph},
    visit::EdgeRef,
};
use thiserror::Error;

use crate::{
    drawing::{
        band::BandFirstSegIndex,
        dot::FixedDotIndex,
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::MakePrimitiveShape,
        rules::RulesTrait,
    },
    geometry::shape::ShapeTrait,
    graph::GetPetgraphIndex,
    layout::Layout,
    router::{
        astar::{astar, AstarError, AstarStrategy, PathTracker},
        draw::DrawException,
        navmesh::{BinavvertexNodeIndex, Navmesh, NavmeshError, NavvertexWeight},
        tracer::{Trace, Tracer},
    },
};

#[derive(Error, Debug, Clone)]
#[error("routing failed")]
pub enum RouterError {
    Navmesh(#[from] NavmeshError),
    Astar(#[from] AstarError),
}

pub struct Router<'a, R: RulesTrait> {
    layout: &'a mut Layout<R>,
    navmesh: Navmesh,
}

struct RouterAstarStrategy<'a, R: RulesTrait> {
    tracer: Tracer<'a, R>,
    trace: Trace,
    target: FixedDotIndex,
}

impl<'a, R: RulesTrait> RouterAstarStrategy<'a, R> {
    pub fn new(tracer: Tracer<'a, R>, trace: Trace, target: FixedDotIndex) -> Self {
        Self {
            tracer,
            trace,
            target,
        }
    }
}

impl<'a, R: RulesTrait> AstarStrategy<&UnGraph<NavvertexWeight, (), usize>, f64, BandFirstSegIndex>
    for RouterAstarStrategy<'a, R>
{
    fn is_goal(
        &mut self,
        graph: &&UnGraph<NavvertexWeight, (), usize>,
        vertex: NodeIndex<usize>,
        tracker: &PathTracker<&UnGraph<NavvertexWeight, (), usize>>,
    ) -> Option<BandFirstSegIndex> {
        let new_path = tracker.reconstruct_path_to(vertex);
        /*.into_iter()
        .map(|ni| graph.node_weight(ni).unwrap().node)
        .collect();*/
        let width = self.trace.width;

        self.tracer
            .rework_path(*graph, &mut self.trace, &new_path[..], width)
            .unwrap();

        self.tracer
            .finish(*graph, &mut self.trace, self.target, width)
            .ok()
    }

    fn edge_cost(
        &mut self,
        graph: &&UnGraph<NavvertexWeight, (), usize>,
        edge: EdgeReference<(), usize>,
    ) -> Option<f64> {
        if edge.target() == self.target.petgraph_index() {
            return None;
        }

        let before_probe_length = 0.0; //self.tracer.layout.band_length(self.trace.head.face());

        let width = self.trace.width;
        let result = self
            .tracer
            .step(*graph, &mut self.trace, edge.target(), width);

        let probe_length = 0.0; //self.tracer.layout.band_length(self.trace.head.face());

        if result.is_ok() {
            self.tracer.undo_step(*graph, &mut self.trace);
            Some(probe_length - before_probe_length)
        } else {
            None
        }
    }

    fn estimate_cost(
        &mut self,
        graph: &&UnGraph<NavvertexWeight, (), usize>,
        vertex: NodeIndex<usize>,
    ) -> f64 {
        let start_point = PrimitiveIndex::from(graph.node_weight(vertex).unwrap().node)
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

    pub fn route_band(&mut self, width: f64) -> Result<BandFirstSegIndex, RouterError> {
        let mut tracer = Tracer::new(self.layout);
        let trace = tracer.start(
            self.navmesh.graph(),
            self.navmesh.source(),
            self.navmesh.source_vertex(),
            width,
        );

        let (_cost, _path, band) = astar(
            self.navmesh.graph(),
            self.navmesh.source_vertex(),
            &mut RouterAstarStrategy::new(tracer, trace, self.navmesh.target()),
        )?;

        Ok(band)
    }

    /*pub fn reroute_band(
        &mut self,
        band: BandIndex,
        to: Point,
        width: f64,
    ) -> Result<BandIndex, RoutingError> {
        {
            let mut layout = self.layout.lock().unwrap();

            layout.remove_band(band);
            layout.move_dot(self.navmesh.to().into(), to).unwrap(); // TODO: Remove `.unwrap()`.
        }

        self.route_band(width)
    }*/

    pub fn layout(&mut self) -> &mut Layout<R> {
        self.layout
    }
}
