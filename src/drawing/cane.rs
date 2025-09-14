// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use super::{
    bend::LooseBendIndex,
    dot::LooseDotIndex,
    primitive::{GetJoints, GetOtherJoint, LooseBendRef, LooseDotRef},
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
    pub fn from_dot(
        dot: LooseDotIndex,
        drawing: &Drawing<impl Clone, impl Copy, impl AccessRules>,
    ) -> Self {
        let bend = LooseDotRef::new(dot, drawing).bend();
        let dot = LooseBendRef::new(bend, drawing).other_joint(dot);
        let seg = LooseDotRef::new(dot, drawing).seg().unwrap();
        Self { bend, dot, seg }
    }
}

impl GetJoints for Cane {
    type F = SeqLooseSegIndex;
    type T = LooseBendIndex;
    fn joints(&self) -> (SeqLooseSegIndex, LooseBendIndex) {
        (self.seg, self.bend)
    }
}
