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
    router::{Router, RouterObserverTrait, RoutingError},
    triangulation::GetVertexIndex,
};

pub struct Autoroute {
    edge_indices: EdgeIndices<usize>,
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

        let (from_dot, to_dot) = {
            let layout = autorouter.router.layout();
            let mut layout = layout.lock().unwrap();

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
        };

        autorouter
            .router
            .route_band(from_dot, to_dot, 100.0, observer);
        Some(())
    }
}

pub struct Autorouter<R: RulesTrait> {
    ratsnest: Ratsnest,
    router: Router<R>,
}

impl<R: RulesTrait> Autorouter<R> {
    pub fn new(layout: Arc<Mutex<Layout<R>>>) -> Result<Self, InsertionError> {
        let ratsnest = Ratsnest::new(&layout.lock().unwrap())?;
        Ok(Self {
            ratsnest,
            router: Router::new(layout),
        })
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
        }
    }

    pub fn router(&self) -> &Router<R> {
        &self.router
    }
}
