use petgraph::stable_graph::StableDiGraph;

use crate::primitive::{Dot, Seg, Bend};
use crate::graph::{TaggedIndex, DotIndex, SegIndex, BendIndex, TaggedWeight, Label, Set};

pub struct Stretch<'a> {
    bend1_dot1: DotIndex,
    bend1: BendIndex,
    bend1_dot2: DotIndex,
    seg1: SegIndex,
    bend2_dot1: DotIndex,
    bend2: BendIndex,
    bend2_dot2: DotIndex,
    seg2: SegIndex,
    bend3_dot1: DotIndex,
    bend3: BendIndex,
    bend3_dot2: DotIndex,
    graph: &'a StableDiGraph<TaggedWeight, Label, usize>,
}

impl<'a> Stretch<'a> {
    pub fn new(index: BendIndex, graph: &'a StableDiGraph<TaggedWeight, Label, usize>) -> Self {
        let bend2 = index;

        let bend2_dot1 = *Bend::new(bend2, graph).prev().unwrap().as_dot().unwrap();
        let seg1 = *Dot::new(bend2_dot1, graph).prev().unwrap().as_seg().unwrap();
        let bend1_dot2 = *Seg::new(seg1, graph).prev().unwrap().as_dot().unwrap();
        let bend1 = *Dot::new(bend1_dot2, graph).prev().unwrap().as_bend().unwrap();
        let bend1_dot1 = *Bend::new(bend1, graph).prev().unwrap().as_dot().unwrap();

        let bend2_dot2 = *Bend::new(bend2, graph).next().unwrap().as_dot().unwrap();
        let seg2 = *Dot::new(bend2_dot1, graph).next().unwrap().as_seg().unwrap();
        let bend3_dot1 = *Seg::new(seg1, graph).next().unwrap().as_dot().unwrap();
        let bend3 = *Dot::new(bend1_dot2, graph).next().unwrap().as_bend().unwrap();
        let bend3_dot2 = *Bend::new(bend1, graph).next().unwrap().as_dot().unwrap();

        Self {
            bend1_dot1,
            bend1,
            bend1_dot2,
            seg1,
            bend2_dot1,
            bend2,
            bend2_dot2,
            seg2,
            bend3_dot1,
            bend3,
            bend3_dot2,
            graph,
        }
    }
}

impl<'a> Set for Stretch<'a> {
    fn interior(&self) -> Vec<TaggedIndex> {
        vec![
            TaggedIndex::Bend(self.bend1),
            TaggedIndex::Dot(self.bend1_dot2),

            TaggedIndex::Seg(self.seg1),

            TaggedIndex::Dot(self.bend2_dot1),
            TaggedIndex::Bend(self.bend2),
            TaggedIndex::Dot(self.bend2_dot2),

            TaggedIndex::Seg(self.seg2),

            TaggedIndex::Dot(self.bend3_dot1),
            TaggedIndex::Bend(self.bend3),
        ]
    }

    fn closure(&self) -> Vec<TaggedIndex> {
        vec![
            TaggedIndex::Dot(self.bend1_dot1),
            TaggedIndex::Bend(self.bend1),
            TaggedIndex::Dot(self.bend1_dot2),

            TaggedIndex::Seg(self.seg1),

            TaggedIndex::Dot(self.bend2_dot1),
            TaggedIndex::Bend(self.bend2),
            TaggedIndex::Dot(self.bend2_dot2),

            TaggedIndex::Seg(self.seg2),

            TaggedIndex::Dot(self.bend3_dot1),
            TaggedIndex::Bend(self.bend3),
            TaggedIndex::Dot(self.bend3_dot2),
        ]
    }

    fn boundary(&self) -> Vec<DotIndex> {
        vec![
            self.bend1_dot1,
            self.bend3_dot2,
        ]
    }
}
