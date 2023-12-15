use petgraph::stable_graph::StableDiGraph;

use crate::{
    graph::{
        GetEnds, Index, Interior, Label, LooseBendIndex, LooseDotIndex, LooseSegIndex, Weight,
    },
    layout::Layout,
    primitive::{GetOtherEnd, LooseBend, LooseDot},
};

#[derive(Debug, Clone, Copy)]
pub struct Segbend {
    pub seg: LooseSegIndex,
    pub dot: LooseDotIndex,
    pub bend: LooseBendIndex,
}

impl Segbend {
    pub fn from_dot(dot: LooseDotIndex, layout: &Layout) -> Self {
        let bend = LooseDot::new(dot, layout).bend();
        let dot = LooseBend::new(bend, layout).other_end(dot);
        let seg = LooseDot::new(dot, layout).seg().unwrap();
        Self { bend, dot, seg }
    }
}

impl Interior<Index> for Segbend {
    fn interior(&self) -> Vec<Index> {
        vec![self.bend.into(), self.dot.into(), self.seg.into()]
    }
}

impl GetEnds<LooseSegIndex, LooseBendIndex> for Segbend {
    fn ends(&self) -> (LooseSegIndex, LooseBendIndex) {
        (self.seg, self.bend)
    }
}
