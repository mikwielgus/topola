// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    board::Board,
    layout::{
        compounds::ComponentId,
        primitives::{JointId, JointSpec, Poly, PolyId, Seg, SegId, SegSpec, Via, ViaId, ViaSpec},
    },
};

impl Board {
    pub fn insert_component(&mut self) -> ComponentId {
        self.layout.insert_component()
    }

    pub fn insert_joint(&mut self, spec: JointSpec) -> JointId {
        self.layout.insert_joint(spec)
    }

    pub fn insert_seg(&mut self, spec: SegSpec) -> SegId {
        self.layout.insert_seg(spec)
    }

    pub fn insert_seg_raw(&mut self, seg: Seg) -> SegId {
        self.layout.insert_seg_raw(seg)
    }

    pub fn insert_via(&mut self, spec: ViaSpec) -> ViaId {
        self.layout.insert_via(spec)
    }

    pub fn insert_via_raw(&mut self, via: Via) -> ViaId {
        self.layout.insert_via_raw(via)
    }

    pub fn insert_poly(&mut self, poly: Poly) -> PolyId {
        self.layout.insert_poly(poly)
    }
}
