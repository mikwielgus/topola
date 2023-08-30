use petgraph::stable_graph::StableDiGraph;

use crate::{
    graph::{BendIndex, DotIndex, Ends, Interior, Label, SegIndex, TaggedIndex, TaggedWeight},
    primitive::{Bend, Dot},
};

pub struct Segbend {
    pub seg: SegIndex,
    pub dot: DotIndex,
    pub bend: BendIndex,
}

impl Segbend {
    pub fn new(bend: BendIndex, dot: DotIndex, seg: SegIndex) -> Self {
        Self { seg, dot, bend }
    }

    pub fn from_dot_prev(
        dot: DotIndex,
        graph: &StableDiGraph<TaggedWeight, Label, usize>,
    ) -> Option<Self> {
        Dot::new(dot, graph).tagged_prev().map(|tagged_prev| {
            let bend = *tagged_prev.as_bend().unwrap();
            let dot = Bend::new(bend, graph).prev().unwrap();
            let seg = *Dot::new(dot, graph)
                .tagged_prev()
                .unwrap()
                .as_seg()
                .unwrap();

            Self { bend, dot, seg }
        })
    }

    pub fn from_dot_next(
        dot: DotIndex,
        graph: &StableDiGraph<TaggedWeight, Label, usize>,
    ) -> Option<Self> {
        Dot::new(dot, graph).tagged_next().map(|tagged_next| {
            let bend = *tagged_next.as_bend().unwrap();
            let dot = Bend::new(bend, graph).next().unwrap();
            let seg = *Dot::new(dot, graph)
                .tagged_next()
                .unwrap()
                .as_seg()
                .unwrap();

            Self { bend, dot, seg }
        })
    }
}

impl Interior<TaggedIndex> for Segbend {
    fn interior(&self) -> Vec<TaggedIndex> {
        vec![
            TaggedIndex::Bend(self.bend),
            TaggedIndex::Dot(self.dot),
            TaggedIndex::Seg(self.seg),
        ]
    }
}

impl Ends<SegIndex, BendIndex> for Segbend {
    fn ends(&self) -> (SegIndex, BendIndex) {
        (self.seg, self.bend)
    }
}
