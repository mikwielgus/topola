// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;

use crate::{
    drawing::{
        bend::{BendIndex, FixedBendIndex, LooseBendIndex},
        dot::FixedDotIndex,
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::{FixedBend, FixedDot, GetFirstGear, LooseBend, Primitive},
        rules::AccessRules,
        Drawing,
    },
    graph::{GetPetgraphIndex, MakeRef},
};

#[enum_dispatch]
pub trait GetNextGear: GetPetgraphIndex {
    fn next_gear(&self) -> Option<LooseBendIndex>;
}

#[enum_dispatch(GetPetgraphIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GearIndex {
    FixedDot(FixedDotIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

impl<'a, CW: 'a, Cel: 'a, R: 'a> MakeRef<'a, Drawing<CW, Cel, R>> for GearIndex {
    type Output = GearRef<'a, CW, Cel, R>;
    fn ref_(&self, drawing: &'a Drawing<CW, Cel, R>) -> GearRef<'a, CW, Cel, R> {
        GearRef::<'a, CW, Cel, R>::new(*self, drawing)
    }
}

impl From<GearIndex> for PrimitiveIndex {
    fn from(wraparoundable: GearIndex) -> Self {
        match wraparoundable {
            GearIndex::FixedDot(dot) => PrimitiveIndex::FixedDot(dot),
            GearIndex::FixedBend(bend) => PrimitiveIndex::FixedBend(bend),
            GearIndex::LooseBend(bend) => PrimitiveIndex::LooseBend(bend),
        }
    }
}

impl From<BendIndex> for GearIndex {
    fn from(bend: BendIndex) -> Self {
        match bend {
            BendIndex::Fixed(bend) => GearIndex::FixedBend(bend),
            BendIndex::Loose(bend) => GearIndex::LooseBend(bend),
        }
    }
}

#[enum_dispatch(GetNextGear, GetDrawing, GetPetgraphIndex)]
pub enum GearRef<'a, CW, Cel, R> {
    FixedDot(FixedDot<'a, CW, Cel, R>),
    FixedBend(FixedBend<'a, CW, Cel, R>),
    LooseBend(LooseBend<'a, CW, Cel, R>),
}

impl<'a, CW, Cel, R> GearRef<'a, CW, Cel, R> {
    pub fn new(index: GearIndex, drawing: &'a Drawing<CW, Cel, R>) -> Self {
        match index {
            GearIndex::FixedDot(dot) => drawing.primitive(dot).into(),
            GearIndex::FixedBend(bend) => drawing.primitive(bend).into(),
            GearIndex::LooseBend(bend) => drawing.primitive(bend).into(),
        }
    }
}

impl<CW, Cel, R> GetNextGear for FixedDot<'_, CW, Cel, R> {
    fn next_gear(&self) -> Option<LooseBendIndex> {
        self.first_gear()
    }
}

impl<CW, Cel, R> GetNextGear for LooseBend<'_, CW, Cel, R> {
    fn next_gear(&self) -> Option<LooseBendIndex> {
        self.outer()
    }
}

impl<CW, Cel, R> GetNextGear for FixedBend<'_, CW, Cel, R> {
    fn next_gear(&self) -> Option<LooseBendIndex> {
        self.first_gear()
    }
}
