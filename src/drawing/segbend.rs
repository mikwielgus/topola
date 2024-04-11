use crate::drawing::{
    bend::LooseBendIndex,
    dot::LooseDotIndex,
    graph::PrimitiveIndex,
    primitive::{GetInterior, GetJoints, GetOtherJoint, LooseBend, LooseDot},
    seg::SeqLooseSegIndex,
    Drawing,
};

use super::rules::RulesTrait;

#[derive(Debug, Clone, Copy)]
pub struct Segbend {
    pub seg: SeqLooseSegIndex,
    pub dot: LooseDotIndex,
    pub bend: LooseBendIndex,
}

impl Segbend {
    pub fn from_dot(dot: LooseDotIndex, drawing: &Drawing<impl Copy, impl RulesTrait>) -> Self {
        let bend = LooseDot::new(dot, drawing).bend();
        let dot = LooseBend::new(bend, drawing).other_joint(dot);
        let seg = LooseDot::new(dot, drawing).seg().unwrap();
        Self { bend, dot, seg }
    }
}

impl GetInterior<PrimitiveIndex> for Segbend {
    fn interior(&self) -> Vec<PrimitiveIndex> {
        vec![self.bend.into(), self.dot.into(), self.seg.into()]
    }
}

impl GetJoints<SeqLooseSegIndex, LooseBendIndex> for Segbend {
    fn joints(&self) -> (SeqLooseSegIndex, LooseBendIndex) {
        (self.seg, self.bend)
    }
}
