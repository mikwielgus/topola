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
    cur_edge: EdgeIndex<usize>,
    navmesh: Navmesh, // Useful for debugging.
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
        let navmesh = Navmesh::new(&layout, from, to).ok()?;

        let this = Self {
            edge_indices,
            cur_edge,
            navmesh,
        };

        Some(this)
    }

    pub fn next<R: RulesTrait>(
        &mut self,
        autorouter: &mut Autorouter<R>,
        observer: &mut impl RouterObserverTrait<R>,
    ) -> Option<()> {
        let (navmesh, from, to) = {
            let (from, to) = self.from_to(autorouter);
            let layout = autorouter.layout.lock().unwrap();
            let navmesh = Navmesh::new(&layout, from, to).ok()?;
            (navmesh, from, to)
        };

        let router = Router::new_with_navmesh(
            &mut autorouter.layout,
            from,
            to,
            std::mem::replace(&mut self.navmesh, navmesh),
        );
        router.unwrap().route_band(to, 100.0, observer);

        if let Some(cur_edge) = self.edge_indices.next() {
            self.cur_edge = cur_edge;
        } else {
            return None;
        }

        Some(())
    }

    pub fn from_to<R: RulesTrait>(
        &self,
        autorouter: &Autorouter<R>,
    ) -> (FixedDotIndex, FixedDotIndex) {
        Self::edge_from_to(autorouter, self.cur_edge)
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

    fn next_navmesh(
        layout: &Layout<impl RulesTrait>,
        from: FixedDotIndex,
        to: FixedDotIndex,
    ) -> Navmesh {
        Navmesh::new(layout, from, to).unwrap()
    }

    pub fn navmesh(&self) -> &Navmesh {
        &self.navmesh
    }
}

pub struct Autorouter<R: RulesTrait> {
    ratsnest: Ratsnest,
    layout: Arc<Mutex<Layout<R>>>,
}

impl<R: RulesTrait> Autorouter<R> {
    pub fn new(layout: Arc<Mutex<Layout<R>>>) -> Result<Self, InsertionError> {
        let ratsnest = Ratsnest::new(&layout.lock().unwrap())?;
        Ok(Self { ratsnest, layout })
    }

    pub fn autoroute(&mut self, layer: u64, observer: &mut impl RouterObserverTrait<R>) {
        if let Some(mut it) = self.autoroute_iter() {
            while let Some(()) = it.next(self, observer) {
                //
            }
        }
    }

    pub fn autoroute_iter(&mut self) -> Option<Autoroute> {
        Autoroute::new(self.ratsnest.graph().edge_indices(), self)
    }

    pub fn layout(&self) -> &Arc<Mutex<Layout<R>>> {
        &self.layout
    }
}
