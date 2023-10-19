use petgraph::stable_graph::StableDiGraph;

use crate::{
    graph::{BendIndex, DotIndex, Ends, Index, Interior, Label, SegIndex, Weight},
    primitive::{Bend, Dot},
};

#[derive(Debug, Clone, Copy)]
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
        graph: &StableDiGraph<Weight, Label, usize>,
    ) -> Option<Self> {
        let bend = *Dot::new(dot, graph).tagged_prev()?.as_bend()?;
        let dot = Bend::new(bend, graph).prev().unwrap();
        let seg = Dot::new(dot, graph)
            .tagged_prev()
            .unwrap()
            .into_seg()
            .unwrap();
        Some(Self { bend, dot, seg })
    }

    pub fn from_dot_next(
        dot: DotIndex,
        graph: &StableDiGraph<Weight, Label, usize>,
    ) -> Option<Self> {
        let bend = *Dot::new(dot, graph).tagged_next()?.as_bend()?;
        let dot = Bend::new(bend, graph).next().unwrap();
        let seg = Dot::new(dot, graph)
            .tagged_next()
            .unwrap()
            .into_seg()
            .unwrap();
        Some(Self { bend, dot, seg })
    }
}

impl Interior<Index> for Segbend {
    fn interior(&self) -> Vec<Index> {
        vec![
            Index::Bend(self.bend),
            Index::Dot(self.dot),
            Index::Seg(self.seg),
        ]
    }
}

impl Ends<SegIndex, BendIndex> for Segbend {
    fn ends(&self) -> (SegIndex, BendIndex) {
        (self.seg, self.bend)
    }
}
