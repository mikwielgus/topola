use std::sync::{Arc, Mutex};

use geo::Point;
use petgraph::{
    graph::EdgeIndices,
    visit::{EdgeRef, IntoEdgeReferences},
};
use spade::InsertionError;

use crate::{
    autorouter::ratsnest::{Ratsnest, RatsnestVertexIndex},
    drawing::{dot::FixedDotIndex, rules::RulesTrait},
    layout::{connectivity::BandIndex, Layout},
    router::{navmesh::Navmesh, Router, RouterObserverTrait, RoutingError},
    triangulation::GetVertexIndex,
};

pub struct Autoroute {
    edge_indices: EdgeIndices<usize>,
    navmesh: Navmesh, // Useful for debugging.
}

impl Autoroute {
    pub fn next<R: RulesTrait>(
        &mut self,
        autorouter: &mut Autorouter<R>,
        observer: &mut impl RouterObserverTrait<R>,
    ) -> Option<()> {
        let Some(ratline) = self.edge_indices.next() else {
            return None;
        };
        let (from, to) = autorouter.ratsnest.graph().edge_endpoints(ratline).unwrap();

        let (navmesh, from_dot, to_dot) = {
            let mut layout = autorouter.layout.lock().unwrap();
            let navmesh = Navmesh::new(&layout).unwrap();

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

            (navmesh, from_dot, to_dot)
        };

        let router = Router::new_with_navmesh(
            &mut autorouter.layout,
            std::mem::replace(&mut self.navmesh, navmesh),
        );
        router
            .unwrap()
            .route_band(from_dot, to_dot, 100.0, observer);
        Some(())
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

    pub fn autoroute(&mut self, observer: &mut impl RouterObserverTrait<R>) {
        let mut it = self.autoroute_iter();
        while let Some(()) = it.next(self, observer) {
            //
        }
    }

    pub fn autoroute_iter(&mut self) -> Autoroute {
        Autoroute {
            edge_indices: self.ratsnest.graph().edge_indices(),
            navmesh: Navmesh::new(&self.layout.lock().unwrap()).unwrap(),
        }
    }

    pub fn layout(&self) -> &Arc<Mutex<Layout<R>>> {
        &self.layout
    }
}
