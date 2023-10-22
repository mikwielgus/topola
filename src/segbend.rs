use petgraph::stable_graph::StableDiGraph;

use crate::{
    graph::{
        BendIndex, DotIndex, Ends, FixedBendIndex, FixedDotIndex, FixedSegIndex, Index, Interior,
        Label, SegIndex, Weight,
    },
    primitive::{FixedBend, FixedDot, TaggedPrevTaggedNext},
};

#[derive(Debug, Clone, Copy)]
pub struct Segbend {
    pub seg: FixedSegIndex,
    pub dot: FixedDotIndex,
    pub bend: FixedBendIndex,
}

impl Segbend {
    pub fn new(bend: FixedBendIndex, dot: FixedDotIndex, seg: FixedSegIndex) -> Self {
        Self { seg, dot, bend }
    }

    pub fn from_dot_prev(
        dot: FixedDotIndex,
        graph: &StableDiGraph<Weight, Label, usize>,
    ) -> Option<Self> {
        let bend = *FixedDot::new(dot, graph).tagged_prev()?.as_fixed_bend()?;
        let dot = FixedBend::new(bend, graph).prev().unwrap();
        let seg = FixedDot::new(dot, graph)
            .tagged_prev()
            .unwrap()
            .into_fixed_seg()
            .unwrap();
        Some(Self { bend, dot, seg })
    }

    pub fn from_dot_next(
        dot: FixedDotIndex,
        graph: &StableDiGraph<Weight, Label, usize>,
    ) -> Option<Self> {
        let bend = *FixedDot::new(dot, graph).tagged_next()?.as_fixed_bend()?;
        let dot = FixedBend::new(bend, graph).next().unwrap();
        let seg = FixedDot::new(dot, graph)
            .tagged_next()
            .unwrap()
            .into_fixed_seg()
            .unwrap();
        Some(Self { bend, dot, seg })
    }
}

impl Interior<Index> for Segbend {
    fn interior(&self) -> Vec<Index> {
        vec![self.bend.into(), self.dot.into(), self.seg.into()]
    }
}

impl Ends<FixedSegIndex, FixedBendIndex> for Segbend {
    fn ends(&self) -> (FixedSegIndex, FixedBendIndex) {
        (self.seg, self.bend)
    }
}
