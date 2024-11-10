use crate::drawing::{
    bend::LooseBendIndex,
    dot::LooseDotIndex,
    graph::PrimitiveIndex,
    primitive::{GetInterior, GetJoints, GetOtherJoint, LooseBend, LooseDot},
    seg::SeqLooseSegIndex,
    Drawing,
};

use super::rules::AccessRules;

#[derive(Debug, Clone, Copy)]
/// Geometrical structure to define Navigation Cord Head
pub struct Cane {
    /// Loose segment of the Cane
    pub seg: SeqLooseSegIndex,
    /// Loose dot of the Cane structure
    pub dot: LooseDotIndex,
    /// Bend of the Cane
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
