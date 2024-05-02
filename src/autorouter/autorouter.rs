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
    pub fn new(layout: Layout<R>) -> Result<Self, InsertionError> {
        Ok(Self {
            ratsnest: Ratsnest::new(&layout)?,
            router: Router::new(layout),
        })
    }

    pub fn autoroute(&mut self, observer: &mut impl RouterObserverTrait<R>) {
        //for ratline in self.overlay.ratsnest().graph().edge_references() {

        // For now, let's only take the first ratline.
        let ratline = self.ratsnest.graph().edge_references().next().unwrap();

        /*self.route(
            self.ratsnest
                .graph()
                .node_weight(ratline.source())
                .unwrap()
                .vertex_index(),
            self.ratsnest
                .graph()
                .node_weight(ratline.target())
                .unwrap()
                .vertex_index(),
            observer,
        );*/

        //}
    }

    fn route(
        &mut self,
        from: RatsnestVertexIndex,
        to: RatsnestVertexIndex,
        observer: &mut impl RouterObserverTrait<R>,
    ) -> Result<BandIndex, RoutingError> {
        let from_dot = self.terminating_dot(from);
        let to_dot = self.terminating_dot(to);

        self.router.route_band(from_dot, to_dot, 3.0, observer)
    }

    fn terminating_dot(&mut self, vertex: RatsnestVertexIndex) -> FixedDotIndex {
        match vertex {
            RatsnestVertexIndex::FixedDot(dot) => dot,
            RatsnestVertexIndex::Zone(zone) => self.router.layout_mut().zone_apex(zone),
        }
    }

    pub fn router(&self) -> &Router<R> {
        &self.router
    }
}
