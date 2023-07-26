use petgraph::stable_graph::StableDiGraph;

use crate::primitive::{Dot, Seg, Bend};
use crate::graph::{TaggedIndex, DotIndex, SegIndex, BendIndex, TaggedWeight, Label, Path};

pub struct Bow<'a> {
    seg1_dot1: DotIndex,
    seg1: SegIndex,
    seg1_dot2: DotIndex,
    bend: BendIndex,
    seg2_dot1: DotIndex,
    seg2: SegIndex,
    seg2_dot2: DotIndex,
    graph: &'a StableDiGraph<TaggedWeight, Label, usize>,
}

impl<'a> Bow<'a> {
    pub fn new(index: BendIndex, graph: &'a StableDiGraph<TaggedWeight, Label, usize>) -> Self {
        let bend = index;

        let seg1_dot2 = *Bend::new(bend, graph).prev().unwrap().as_dot().unwrap();
        let seg1 = *Dot::new(seg1_dot2, graph).prev().unwrap().as_seg().unwrap();
        let seg1_dot1 = *Seg::new(seg1, graph).prev().unwrap().as_dot().unwrap();

        let seg2_dot1 = *Bend::new(bend, graph).next().unwrap().as_dot().unwrap();
        let seg2 = *Dot::new(seg2_dot1, graph).next().unwrap().as_seg().unwrap();
        let seg2_dot2 = *Seg::new(seg2, graph).next().unwrap().as_dot().unwrap();

        Self {
            seg1_dot1,
            seg1,
            seg1_dot2,
            bend,
            seg2_dot1,
            seg2,
            seg2_dot2,
            graph,
        }
    }
}

impl<'a> Path for Bow<'a> {
    fn interior(&self) -> Vec<TaggedIndex> {
        vec![
            TaggedIndex::Seg(self.seg1),

            TaggedIndex::Dot(self.seg1_dot2),
            TaggedIndex::Bend(self.bend),
            TaggedIndex::Dot(self.seg2_dot1),

            TaggedIndex::Seg(self.seg2),
        ]
    }

    fn closure(&self) -> Vec<TaggedIndex> {
        vec![
            TaggedIndex::Dot(self.seg1_dot1),
            TaggedIndex::Seg(self.seg1),

            TaggedIndex::Dot(self.seg1_dot2),
            TaggedIndex::Bend(self.bend),
            TaggedIndex::Dot(self.seg2_dot1),

            TaggedIndex::Seg(self.seg2),
            TaggedIndex::Dot(self.seg2_dot2),
        ]
    }

    fn ends(&self) -> [DotIndex; 2] {
        [self.seg1_dot1, self.seg2_dot2]
    }
}
