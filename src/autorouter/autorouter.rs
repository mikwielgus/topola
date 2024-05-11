use std::{
    iter::Peekable,
    sync::{Arc, Mutex},
};

use geo::Point;
use petgraph::{
    graph::{EdgeIndex, EdgeIndices, NodeIndex},
    visit::{EdgeRef, IntoEdgeReferences},
};
use spade::InsertionError;

use crate::{
    autorouter::ratsnest::{Ratsnest, RatsnestVertexIndex},
    drawing::{
        dot::FixedDotIndex,
        graph::{GetLayer, GetMaybeNet},
        rules::RulesTrait,
    },
    layout::{connectivity::BandIndex, Layout},
    router::{navmesh::Navmesh, Router, RouterObserverTrait, RoutingError},
    triangulation::GetVertexIndex,
};

pub struct Autoroute {
    edge_indices: EdgeIndices<usize>,
    navmesh: Option<Navmesh>, // Useful for debugging.
}

impl Autoroute {
    pub fn new(
        mut edge_indices: EdgeIndices<usize>,
        autorouter: &mut Autorouter<impl RulesTrait>,
    ) -> Option<Self> {
        let Some(cur_edge) = edge_indices.next() else {
            return None;
        };

        let (from, to) = Self::edge_from_to(autorouter, cur_edge);
        let layout = autorouter.layout.lock().unwrap();
        let navmesh = Some(Navmesh::new(&layout, from, to).ok()?);

        let this = Self {
            edge_indices,
            navmesh,
        };

        Some(this)
    }

    pub fn next<R: RulesTrait>(
        &mut self,
        autorouter: &mut Autorouter<R>,
        observer: &mut impl RouterObserverTrait<R>,
    ) -> bool {
        let new_navmesh = if let Some(cur_edge) = self.edge_indices.next() {
            let (from, to) = Self::edge_from_to(autorouter, cur_edge);

            let layout = autorouter.layout.lock().unwrap();
            Some(Navmesh::new(&layout, from, to).ok().unwrap())
        } else {
            None
        };

        let router = Router::new_from_navmesh(
            &mut autorouter.layout,
            std::mem::replace(&mut self.navmesh, new_navmesh).unwrap(),
        );
        router.unwrap().route_band(100.0, observer);

        self.navmesh.is_some()
    }

    fn edge_from_to<R: RulesTrait>(
        autorouter: &Autorouter<R>,
        edge: EdgeIndex<usize>,
    ) -> (FixedDotIndex, FixedDotIndex) {
        let mut layout = autorouter.layout.lock().unwrap();
        let (from, to) = autorouter.ratsnest.graph().edge_endpoints(edge).unwrap();

        let from_dot = match autorouter
            .ratsnest
            .graph()
            .node_weight(from)
            .unwrap()
            .vertex_index()
        {
            RatsnestVertexIndex::FixedDot(dot) => dot,
            RatsnestVertexIndex::Zone(zone) => layout.zone_apex(zone),
        };

        let to_dot = match autorouter
            .ratsnest
            .graph()
            .node_weight(to)
            .unwrap()
            .vertex_index()
        {
            RatsnestVertexIndex::FixedDot(dot) => dot,
            RatsnestVertexIndex::Zone(zone) => layout.zone_apex(zone),
        };

        (from_dot, to_dot)
    }

    pub fn navmesh(&self) -> &Option<Navmesh> {
        &self.navmesh
    }
}

pub struct Autorouter<R: RulesTrait> {
    layout: Arc<Mutex<Layout<R>>>,
    ratsnest: Ratsnest,
}

impl<R: RulesTrait> Autorouter<R> {
    pub fn new(layout: Arc<Mutex<Layout<R>>>) -> Result<Self, InsertionError> {
        let ratsnest = Ratsnest::new(&layout.lock().unwrap())?;
        Ok(Self { layout, ratsnest })
    }

    pub fn autoroute(&mut self, layer: u64, observer: &mut impl RouterObserverTrait<R>) {
        if let Some(mut autoroute) = self.autoroute_walk() {
            while autoroute.next(self, observer) {
                //
            }
        }
    }

    pub fn autoroute_walk(&mut self) -> Option<Autoroute> {
        Autoroute::new(self.ratsnest.graph().edge_indices(), self)
    }

    pub fn layout(&self) -> &Arc<Mutex<Layout<R>>> {
        &self.layout
    }

    pub fn ratsnest(&self) -> &Ratsnest {
        &self.ratsnest
    }
}
