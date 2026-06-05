// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::From;
use serde::{Deserialize, Serialize};

mod joint;
mod polygon;
mod segment;
mod via;

pub use joint::*;
pub use polygon::*;
pub use segment::*;
pub use via::*;

use crate::{
    Rect2,
    layout::{Layout, compounds::PinId},
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, From, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PrimitiveId {
    Joint(JointId),
    Segment(SegmentId),
    Via(ViaId),
    Polygon(PolygonId),
}

impl Layout {
    pub fn primitive_pin(&self, primitive: PrimitiveId) -> Option<PinId> {
        match primitive {
            PrimitiveId::Joint(joint_id) => self.joint(joint_id).spec.pin,
            PrimitiveId::Segment(segment_id) => self.segment(segment_id).spec.pin,
            PrimitiveId::Via(via_id) => self.via(via_id).spec.pin,
            PrimitiveId::Polygon(polygon_id) => self.polygon(polygon_id).pin,
        }
    }

    pub fn primitive_bbox2(&self, primitive: PrimitiveId) -> Rect2<i64> {
        match primitive {
            PrimitiveId::Joint(joint_id) => self.joint(joint_id).bbox().xy(),
            PrimitiveId::Segment(segment_id) => self.segment(segment_id).bbox().xy(),
            PrimitiveId::Via(via_id) => self.via(via_id).bbox().xy(),
            PrimitiveId::Polygon(polygon_id) => self.polygon(polygon_id).bbox().xy(),
        }
    }
}
