use geo::Point;
use petgraph::stable_graph::StableDiGraph;

use crate::{
    drawing::{
        bend::LooseBendWeight,
        dot::{DotIndex, FixedDotIndex, LooseDotIndex, LooseDotWeight},
        rules::RulesTrait,
        seg::{LoneLooseSegIndex, LoneLooseSegWeight, SeqLooseSegIndex, SeqLooseSegWeight},
        segbend::Segbend,
        Drawing, Infringement, LayoutException,
    },
    graph::GetNodeIndex,
    wraparoundable::WraparoundableIndex,
};

use super::connectivity::{
    BandIndex, BandWeight, ConnectivityLabel, ConnectivityWeight, ContinentIndex,
};

pub struct Layout<R: RulesTrait> {
    drawing: Drawing<R>, // Shouldn't be public, but is for now because `Draw` needs it.
    connectivity: StableDiGraph<ConnectivityWeight, ConnectivityLabel, usize>,
}

impl<R: RulesTrait> Layout<R> {
    pub fn new(drawing: Drawing<R>) -> Self {
        Self {
            drawing,
            connectivity: StableDiGraph::default(),
        }
    }

    pub fn remove_band(&mut self, band: BandIndex) {
        todo!()
    }

    pub fn remove_segbend(&mut self, segbend: &Segbend, face: LooseDotIndex) {
        self.drawing.remove_segbend(segbend, face)
    }

    pub fn start_band(&mut self, from: FixedDotIndex) -> BandIndex {
        let band = self
            .connectivity
            .add_node(ConnectivityWeight::Band(BandWeight { from, to: None }));
        self.connectivity.update_edge(
            self.continent(from.into()).node_index(),
            band,
            ConnectivityLabel::Band,
        );
        BandIndex::new(band)
    }

    pub fn finish_band(&mut self, band: BandIndex, to: FixedDotIndex) {
        self.connectivity.update_edge(
            band.node_index(),
            self.continent(to.into()).node_index(),
            ConnectivityLabel::Band,
        );
    }

    pub fn insert_segbend(
        &mut self,
        from: DotIndex,
        around: WraparoundableIndex,
        dot_weight: LooseDotWeight,
        seg_weight: SeqLooseSegWeight,
        bend_weight: LooseBendWeight,
        cw: bool,
    ) -> Result<Segbend, LayoutException> {
        self.drawing
            .insert_segbend(from, around, dot_weight, seg_weight, bend_weight, cw)
    }

    pub fn add_lone_loose_seg(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: LoneLooseSegWeight,
    ) -> Result<LoneLooseSegIndex, Infringement> {
        self.drawing.add_lone_loose_seg(from, to, weight)
    }

    pub fn add_seq_loose_seg(
        &mut self,
        from: DotIndex,
        to: LooseDotIndex,
        weight: SeqLooseSegWeight,
    ) -> Result<SeqLooseSegIndex, Infringement> {
        self.drawing.add_seq_loose_seg(from, to, weight)
    }

    pub fn move_dot(&mut self, dot: DotIndex, to: Point) -> Result<(), Infringement> {
        self.drawing.move_dot(dot, to)
    }

    pub fn band_from(&self, band: BandIndex) -> FixedDotIndex {
        todo!()
    }

    pub fn band_to(&self, band: BandIndex) -> Option<FixedDotIndex> {
        todo!()
    }

    pub fn band_length(&self, band: BandIndex) -> f64 {
        // TODO.
        0.0
    }

    pub fn layout(&self) -> &Drawing<R> {
        &self.drawing
    }

    pub fn continent(&self, dot: FixedDotIndex) -> ContinentIndex {
        // TODO.
        ContinentIndex::new(0.into())
    }
}
