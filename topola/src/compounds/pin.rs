// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::Constructor;
use serde::{Deserialize, Serialize};

use crate::primitives::{JointId, PolygonId, SegmentId, ViaId};

#[derive(
    Clone, Constructor, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct PinId(usize);

impl PinId {
    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug)]
pub struct Pin {
    pub(crate) joints: Vec<JointId>,
    pub(crate) segments: Vec<SegmentId>,
    pub(crate) vias: Vec<ViaId>,
    pub(crate) polygons: Vec<PolygonId>,
}

impl Pin {
    pub fn new() -> Self {
        Self {
            joints: Vec::new(),
            segments: Vec::new(),
            vias: Vec::new(),
            polygons: Vec::new(),
        }
    }
}
