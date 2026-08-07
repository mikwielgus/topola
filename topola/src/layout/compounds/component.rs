// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};

use crate::layout::{
    compounds::PinId,
    primitives::{JointId, PolyId, PrimitiveId, SegId, ViaId},
};

#[derive(
    Clone,
    Constructor,
    Copy,
    Debug,
    Default,
    Deserialize,
    Eq,
    From,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
)]
pub struct ComponentId(usize);

impl ComponentId {
    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug, Default)]
pub struct Component {
    pub pins: Vec<PinId>,
    pub joints: Vec<JointId>,
    pub segs: Vec<SegId>,
    pub vias: Vec<ViaId>,
    pub polys: Vec<PolyId>,
}

impl Component {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn primitives(&self) -> impl Iterator<Item = PrimitiveId> + '_ {
        self.joints
            .iter()
            .map(|&joint_id| PrimitiveId::Joint(joint_id))
            .chain(self.segs.iter().map(|&seg_id| PrimitiveId::Seg(seg_id)))
            .chain(self.vias.iter().map(|&via_id| PrimitiveId::Via(via_id)))
            .chain(self.polys.iter().map(|&poly_id| PrimitiveId::Poly(poly_id)))
    }
}
