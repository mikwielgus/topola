use super::{
    bend::LooseBendIndex,
    dot::LooseDotIndex,
    graph::PrimitiveIndex,
    primitive::{GetInterior, GetJoints, GetOtherJoint, LooseBend, LooseDot},
    rules::AccessRules,
    seg::SeqLooseSegIndex,
    Drawing,
};

/// A cane is a sequence consisting of a seg followed by a dot followed by a
/// bend, with the dot joining the seg with the bend as a joint.
///
/// The name "cane" comes from the iconic cane: a slender walking stick that
/// ends with a round bend that is its handle. Topola's canes are of similar
/// shape.
#[derive(Debug, Clone, Copy)]
pub struct Cane {
    pub seg: SeqLooseSegIndex,
    pub dot: LooseDotIndex,
    pub bend: LooseBendIndex,
}

impl Cane {
    pub fn from_dot(dot: LooseDotIndex, drawing: &Drawing<impl Copy, impl AccessRules>) -> Self {
        let bend = LooseDot::new(dot, drawing).bend();
        let dot = LooseBend::new(bend, drawing).other_joint(dot);
        let seg = LooseDot::new(dot, drawing).seg().unwrap();
        Self { bend, dot, seg }
    }
}

impl GetInterior<PrimitiveIndex> for Cane {
    fn interior(&self) -> Vec<PrimitiveIndex> {
        vec![self.bend.into(), self.dot.into(), self.seg.into()]
    }
}

impl GetJoints<SeqLooseSegIndex, LooseBendIndex> for Cane {
    fn joints(&self) -> (SeqLooseSegIndex, LooseBendIndex) {
        (self.seg, self.bend)
    }
}
