use petgraph::stable_graph::StableDiGraph;

use crate::{
    graph::{
        FixedBendIndex, FixedDotIndex, FixedSegIndex, GetEnds, Index, Interior, Label, Weight,
    },
    primitive::{FixedBend, FixedDot},
};

#[derive(Debug, Clone, Copy)]
pub struct Segbend {
    pub seg: FixedSegIndex,
    pub dot: FixedDotIndex,
    pub bend: FixedBendIndex,
}

impl Segbend {
    pub fn from_dot(
        dot: FixedDotIndex,
        graph: &StableDiGraph<Weight, Label, usize>,
    ) -> Option<Self> {
        let bend = FixedDot::new(dot, graph).bend()?;
        let dot = FixedBend::new(bend, graph).other_end(dot);
        let seg = FixedDot::new(dot, graph).seg()?;
        Some(Self { bend, dot, seg })
    }
}

impl Interior<Index> for Segbend {
    fn interior(&self) -> Vec<Index> {
        vec![self.bend.into(), self.dot.into(), self.seg.into()]
    }
}

impl GetEnds<FixedSegIndex, FixedBendIndex> for Segbend {
    fn ends(&self) -> (FixedSegIndex, FixedBendIndex) {
        (self.seg, self.bend)
    }
}
