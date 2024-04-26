use geo::Point;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use spade::InsertionError;

use crate::{
    autorouter::ratsnest::{Ratsnest, RatsnestVertexIndex},
    drawing::rules::RulesTrait,
    layout::Layout,
    router::Router,
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

    pub fn autoroute(&mut self) {
        //for ratline in self.overlay.ratsnest().graph().edge_references() {

        // For now, let's only take the first ratline.
        let ratline = self.ratsnest.graph().edge_references().next().unwrap();

        self.route(
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
        );

        //}
    }

    fn route(&mut self, from: RatsnestVertexIndex, to: RatsnestVertexIndex) {
        todo!();
    }

    pub fn router(&self) -> &Router<R> {
        &self.router
    }
}
