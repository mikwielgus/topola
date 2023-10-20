use petgraph::stable_graph::StableDiGraph;

use crate::graph::{BendIndex, DotIndex, Ends, Index, Interior, Label, SegIndex, Weight};
use crate::primitive::{Bend, Dot, Seg, TaggedPrevTaggedNext};

#[derive(Debug, Clone, Copy)]
pub struct Bow {
    seg1_dot1: DotIndex,
    seg1: SegIndex,
    seg1_dot2: DotIndex,
    bend: BendIndex,
    seg2_dot1: DotIndex,
    seg2: SegIndex,
    seg2_dot2: DotIndex,
}

impl Bow {
    pub fn from_bend(index: BendIndex, graph: &StableDiGraph<Weight, Label, usize>) -> Self {
        let bend = index;

        let seg1_dot2 = Bend::new(bend, graph).prev().unwrap();
        let seg1 = Dot::new(seg1_dot2, graph)
            .tagged_prev()
            .unwrap()
            .into_seg()
            .unwrap();
        let seg1_dot1 = Seg::new(seg1, graph).prev().unwrap();

        let seg2_dot1 = Bend::new(bend, graph).next().unwrap();
        let seg2 = Dot::new(seg2_dot1, graph)
            .tagged_next()
            .unwrap()
            .into_seg()
            .unwrap();
        let seg2_dot2 = Seg::new(seg2, graph).next().unwrap();

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
            Index::Seg(self.seg1),
            Index::Dot(self.seg1_dot2),
            Index::Bend(self.bend),
            Index::Dot(self.seg2_dot1),
            Index::Seg(self.seg2),
        ]
    }
}

impl Ends<DotIndex, DotIndex> for Bow {
    fn ends(&self) -> (DotIndex, DotIndex) {
        (self.seg1_dot1, self.seg2_dot2)
    }
}
