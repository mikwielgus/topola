use geo::Point;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use spade::InsertionError;

use crate::{
    autorouter::{overlay::Overlay, ratsnest::RatsnestVertexIndex},
    drawing::rules::RulesTrait,
    layout::Layout,
    router::Router,
    triangulation::GetVertexIndex,
};

pub struct Autorouter<R: RulesTrait> {
    overlay: Overlay,
    router: Router<R>,
}

impl<R: RulesTrait> Autorouter<R> {
    pub fn new(layout: Layout<R>) -> Result<Self, InsertionError> {
        Ok(Self {
            overlay: Overlay::new(&layout)?,
            router: Router::new(layout),
        })
    }

    pub fn autoroute(&mut self) {
        //for ratline in self.overlay.ratsnest().graph().edge_references() {

        // For now, let's only take the first ratline.
        let ratline = self
            .overlay
            .ratsnest()
            .graph()
            .edge_references()
            .next()
            .unwrap();

        self.route(
            self.overlay
                .ratsnest()
                .graph()
                .node_weight(ratline.source())
                .unwrap()
                .vertex_index(),
            self.overlay
                .ratsnest()
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

    // TODO: Move somewhere higher in abstraction.
    pub fn click(&mut self, at: Point) {
        self.overlay.click(self.router.layout(), at);
    }

    pub fn overlay(&self) -> &Overlay {
        &self.overlay
    }

    pub fn router(&self) -> &Router<R> {
        &self.router
    }
}
