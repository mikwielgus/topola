// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};

use crate::layout::primitives::{JointId, PolygonId, SegmentId, ViaId};

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
    pub joints: Vec<JointId>,
    pub segments: Vec<SegmentId>,
    pub vias: Vec<ViaId>,
    pub polygons: Vec<PolygonId>,
}

impl Component {
    pub fn new() -> Self {
        Default::default()
    }
}
