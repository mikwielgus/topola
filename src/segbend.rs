use petgraph::stable_graph::StableDiGraph;

use crate::{
    graph::{BendIndex, DotIndex, Interior, Label, SegIndex, TaggedIndex, TaggedWeight},
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
        if let Some(tagged_prev) = Dot::new(dot, graph).tagged_prev() {
            let bend = *tagged_prev.as_bend().unwrap();
            let dot = Bend::new(bend, graph).prev().unwrap();
            let seg = *Dot::new(dot, graph)
                .tagged_prev()
                .unwrap()
                .as_seg()
                .unwrap();

            Some(Self { bend, dot, seg })
        } else {
            None
        }
    }

    pub fn from_dot_next(
        dot: DotIndex,
        graph: &StableDiGraph<TaggedWeight, Label, usize>,
    ) -> Option<Self> {
        if let Some(tagged_prev) = Dot::new(dot, graph).tagged_next() {
            let bend = *tagged_prev.as_bend().unwrap();
            let dot = Bend::new(bend, graph).next().unwrap();
            let seg = *Dot::new(dot, graph)
                .tagged_next()
                .unwrap()
                .as_seg()
                .unwrap();

            Some(Self { bend, dot, seg })
        } else {
            None
        }
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
