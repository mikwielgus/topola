// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::From;
use serde::{Deserialize, Serialize};

mod joint;
mod poly;
mod seg;
mod via;

pub use joint::*;
pub use poly::*;
pub use seg::*;
pub use via::*;

use crate::{
    Rect2,
    layout::{Layout, compounds::PinId},
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, From, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PrimitiveId {
    Joint(JointId),
    Seg(SegId),
    Via(ViaId),
    Poly(PolyId),
}

impl Layout {
    pub fn primitive_pin(&self, primitive: PrimitiveId) -> Option<PinId> {
        match primitive {
            PrimitiveId::Joint(joint_id) => self.joint(joint_id).spec.pin,
            PrimitiveId::Seg(seg_id) => self.seg(seg_id).spec.pin,
            PrimitiveId::Via(via_id) => self.via(via_id).spec.pin,
            PrimitiveId::Poly(poly_id) => self.poly(poly_id).spec.pin,
        }
    }

    pub fn primitive_bbox2(&self, primitive: PrimitiveId) -> Rect2<i64> {
        match primitive {
            PrimitiveId::Joint(joint_id) => self.joint(joint_id).bbox().xy(),
            PrimitiveId::Seg(seg_id) => self.seg(seg_id).bbox().xy(),
            PrimitiveId::Via(via_id) => self.via(via_id).bbox().xy(),
            PrimitiveId::Poly(poly_id) => self.poly(poly_id).bbox().xy(),
        }
    }
}
