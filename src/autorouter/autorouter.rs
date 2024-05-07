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
    drawing::{dot::FixedDotIndex, graph::GetLayer, rules::RulesTrait},
    layout::{connectivity::BandIndex, Layout},
    router::{navmesh::Navmesh, Router, RouterObserverTrait, RoutingError},
    triangulation::GetVertexIndex,
};

pub struct Autoroute {
    edge_indices: Peekable<EdgeIndices<usize>>,
    navmesh: Navmesh, // Useful for debugging.
}

impl Autoroute {
    pub fn new(
        edge_indices: EdgeIndices<usize>,
        autorouter: &mut Autorouter<impl RulesTrait>,
    ) -> Option<Self> {
        let mut peekable_edge_indices = edge_indices.peekable();
        let Some(ratline) = peekable_edge_indices.peek() else {
            return None;
        };

        let mut layout = autorouter.layout.lock().unwrap();
        let (from_dot, to_dot) = Self::terminating_dots(autorouter, &mut layout, ratline);
        let navmesh = Self::next_navmesh(&layout, from_dot);
        Some(Self {
            edge_indices: peekable_edge_indices,
            navmesh,
        })
    }

    pub fn next<R: RulesTrait>(
        &mut self,
        autorouter: &mut Autorouter<R>,
        observer: &mut impl RouterObserverTrait<R>,
    ) -> Option<()> {
        let Some(ratline) = self.edge_indices.next() else {
            return None;
        };

        let (navmesh, from_dot, to_dot) = {
            let mut layout = autorouter.layout.lock().unwrap();
            let (from_dot, to_dot) = Self::terminating_dots(autorouter, &mut layout, &ratline);
            let navmesh = Self::next_navmesh(&layout, from_dot);
            (navmesh, from_dot, to_dot)
        };

        let router = Router::new_with_navmesh(
            &mut autorouter.layout,
            from_dot,
            std::mem::replace(&mut self.navmesh, navmesh),
        );
        router.unwrap().route_band(to_dot, 100.0, observer);
        Some(())
    }

    fn terminating_dots<R: RulesTrait>(
        autorouter: &Autorouter<R>,
        layout: &mut Layout<R>,
        ratline: &EdgeIndex<usize>,
    ) -> (FixedDotIndex, FixedDotIndex) {
        let (from, to) = autorouter
            .ratsnest
            .graph()
            .edge_endpoints(*ratline)
            .unwrap();

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

    fn next_navmesh(layout: &Layout<impl RulesTrait>, from: FixedDotIndex) -> Navmesh {
        let layer = layout.drawing().primitive(from).layer();
        Navmesh::new(layout, layer).unwrap()
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
