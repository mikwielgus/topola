use petgraph::stable_graph::StableDiGraph;

use crate::graph::{
    FixedBendIndex, FixedDotIndex, FixedSegIndex, GetEnds, Index, Interior, Label, Weight,
};
use crate::primitive::{FixedBend, FixedDot, FixedSeg};

#[derive(Debug, Clone, Copy)]
pub struct Bow {
    seg1_dot1: FixedDotIndex,
    seg1: FixedSegIndex,
    seg1_dot2: FixedDotIndex,
    bend: FixedBendIndex,
    seg2_dot1: FixedDotIndex,
    seg2: FixedSegIndex,
    seg2_dot2: FixedDotIndex,
}

impl Bow {
    pub fn from_bend(index: FixedBendIndex, graph: &StableDiGraph<Weight, Label, usize>) -> Self {
        let bend = index;

        let seg1_dot2 = FixedBend::new(bend, graph).ends().0;
        let seg1 = FixedDot::new(seg1_dot2, graph).seg().unwrap();
        let seg1_dot1 = FixedSeg::new(seg1, graph).other_end(seg1_dot2);

        let seg2_dot1 = FixedBend::new(bend, graph).ends().1;
        let seg2 = FixedDot::new(seg2_dot1, graph).seg().unwrap();
        let seg2_dot2 = FixedSeg::new(seg2, graph).other_end(seg2_dot1);

        Self {
            seg1_dot1,
            seg1,
            seg1_dot2,
            bend,
            seg2_dot1,
            seg2,
            seg2_dot2,
        }
    }
}

impl Interior<Index> for Bow {
    fn interior(&self) -> Vec<Index> {
        vec![
            self.seg1.into(),
            self.seg1_dot2.into(),
            self.bend.into(),
            self.seg2_dot1.into(),
            self.seg2.into(),
        ]
    }
}

impl GetEnds<FixedDotIndex, FixedDotIndex> for Bow {
    fn ends(&self) -> (FixedDotIndex, FixedDotIndex) {
        (self.seg1_dot1, self.seg2_dot2)
    }
}
