use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;

use crate::{
    drawing::{
        bend::{BendIndex, FixedBendIndex, LooseBendIndex},
        dot::FixedDotIndex,
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::{FixedBend, FixedDot, GetFirstGear, GetInnerOuter, LooseBend, Primitive},
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

impl<'a, CW: Copy, R: AccessRules> MakeRef<'a, GearRef<'a, CW, R>, Drawing<CW, R>> for GearIndex {
    fn ref_(&self, drawing: &'a Drawing<CW, R>) -> GearRef<'a, CW, R> {
        GearRef::new(*self, drawing)
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
pub enum GearRef<'a, CW: Copy, R: AccessRules> {
    FixedDot(FixedDot<'a, CW, R>),
    FixedBend(FixedBend<'a, CW, R>),
    LooseBend(LooseBend<'a, CW, R>),
}

impl<'a, CW: Copy, R: AccessRules> GearRef<'a, CW, R> {
    pub fn new(index: GearIndex, drawing: &'a Drawing<CW, R>) -> Self {
        match index {
            GearIndex::FixedDot(dot) => drawing.primitive(dot).into(),
            GearIndex::FixedBend(bend) => drawing.primitive(bend).into(),
            GearIndex::LooseBend(bend) => drawing.primitive(bend).into(),
        }
    }
}

impl<'a, CW: Copy, R: AccessRules> GetNextGear for FixedDot<'a, CW, R> {
    fn next_gear(&self) -> Option<LooseBendIndex> {
        self.first_gear()
    }
}

impl<'a, CW: Copy, R: AccessRules> GetNextGear for LooseBend<'a, CW, R> {
    fn next_gear(&self) -> Option<LooseBendIndex> {
        self.outer()
    }
}

impl<'a, CW: Copy, R: AccessRules> GetNextGear for FixedBend<'a, CW, R> {
    fn next_gear(&self) -> Option<LooseBendIndex> {
        self.first_gear()
    }
}
