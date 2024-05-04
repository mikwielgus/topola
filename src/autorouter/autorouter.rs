use std::sync::{Arc, Mutex};

use geo::Point;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use spade::InsertionError;

use crate::{
    autorouter::ratsnest::{Ratsnest, RatsnestVertexIndex},
    drawing::{dot::FixedDotIndex, rules::RulesTrait},
    layout::{connectivity::BandIndex, Layout},
    router::{Router, RouterObserverTrait, RoutingError},
    triangulation::GetVertexIndex,
};

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
        for ratline in self.ratsnest.graph().edge_references() {
            let from = self
                .ratsnest
                .graph()
                .node_weight(ratline.source())
                .unwrap()
                .vertex_index();
            let to = self
                .ratsnest
                .graph()
                .node_weight(ratline.target())
                .unwrap()
                .vertex_index();

            let (from_dot, to_dot) = {
                let layout = self.router.layout();
                let mut layout = layout.lock().unwrap();

                let from_dot = match from {
                    RatsnestVertexIndex::FixedDot(dot) => dot,
                    RatsnestVertexIndex::Zone(zone) => layout.zone_apex(zone),
                };

                let to_dot = match to {
                    RatsnestVertexIndex::FixedDot(dot) => dot,
                    RatsnestVertexIndex::Zone(zone) => layout.zone_apex(zone),
                };

                (from_dot, to_dot)
            };

            self.router.route_band(from_dot, to_dot, 100.0, observer);
        }
    }

    pub fn router(&self) -> &Router<R> {
        &self.router
    }
}
