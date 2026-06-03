// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};

use crate::{
    Layout, Vector2,
    layout::{
        compounds::{ComponentId, NetId},
        primitives::{JointId, PolygonId, SegmentId, ViaId},
    },
    primitives::PrimitiveId,
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
pub struct PinId(usize);

impl PinId {
    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PinSpec {
    pub component: Option<ComponentId>,
    pub net: Option<NetId>,
}

#[derive(Clone, Debug)]
pub struct Pin {
    pub spec: PinSpec,
    pub joints: Vec<JointId>,
    pub segments: Vec<SegmentId>,
    pub vias: Vec<ViaId>,
    pub polygons: Vec<PolygonId>,
}

impl Pin {
    pub fn new(spec: PinSpec) -> Self {
        Pin {
            spec,
            joints: Vec::new(),
            segments: Vec::new(),
            vias: Vec::new(),
            polygons: Vec::new(),
        }
    }

    pub fn primitives(&self) -> impl Iterator<Item = PrimitiveId> + '_ {
        self.joints
            .iter()
            .map(|&joint_id| PrimitiveId::Joint(joint_id))
            .chain(
                self.segments
                    .iter()
                    .map(|&segment_id| PrimitiveId::Segment(segment_id)),
            )
            .chain(self.vias.iter().map(|&via_id| PrimitiveId::Via(via_id)))
            .chain(
                self.polygons
                    .iter()
                    .map(|&polygon_id| PrimitiveId::Polygon(polygon_id)),
            )
    }
}

impl Layout {
    pub fn pin_centroid(&self, pin_id: PinId) -> Vector2<i64> {
        let pin = self.pin(pin_id);
        let mut sum = Vector2::new(0, 0);
        let mut count = 0;

        for &joint_id in &pin.joints {
            sum = sum + self.joint(joint_id).center();
            count += 1;
        }
        for &segment_id in &pin.segments {
            sum = sum + self.segment(segment_id).center();
            count += 1;
        }
        for &via_id in &pin.vias {
            sum = sum + self.via(via_id).position;
            count += 1;
        }
        for &polygon_id in &pin.polygons {
            sum = sum + self.polygon(polygon_id).center();
            count += 1;
        }

        if count == 0 {
            return Vector2::new(0, 0);
        }

        let count = count as i64;
        Vector2::new(sum.x / count, sum.y / count)
    }
}
