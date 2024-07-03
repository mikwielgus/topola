use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;

use crate::{
    drawing::{
        bend::{BendIndex, FixedBendIndex, LooseBendIndex},
        dot::FixedDotIndex,
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::{FixedBend, FixedDot, GetFirstRail, GetInnerOuter, LooseBend, Primitive},
        rules::AccessRules,
        Drawing,
    },
    graph::GetPetgraphIndex,
};

#[enum_dispatch]
pub trait GetWraparound: GetPetgraphIndex {
    fn wraparound(&self) -> Option<LooseBendIndex>;
}

#[enum_dispatch(GetPetgraphIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WraparoundableIndex {
    FixedDot(FixedDotIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

impl From<WraparoundableIndex> for PrimitiveIndex {
    fn from(wraparoundable: WraparoundableIndex) -> Self {
        match wraparoundable {
            WraparoundableIndex::FixedDot(dot) => PrimitiveIndex::FixedDot(dot),
            WraparoundableIndex::FixedBend(bend) => PrimitiveIndex::FixedBend(bend),
            WraparoundableIndex::LooseBend(bend) => PrimitiveIndex::LooseBend(bend),
        }
    }
}

impl From<BendIndex> for WraparoundableIndex {
    fn from(bend: BendIndex) -> Self {
        match bend {
            BendIndex::Fixed(bend) => WraparoundableIndex::FixedBend(bend),
            BendIndex::Loose(bend) => WraparoundableIndex::LooseBend(bend),
        }
    }
}

#[enum_dispatch(GetWraparound, GetDrawing, GetPetgraphIndex)]
pub enum Wraparoundable<'a, CW: Copy, R: AccessRules> {
    FixedDot(FixedDot<'a, CW, R>),
    FixedBend(FixedBend<'a, CW, R>),
    LooseBend(LooseBend<'a, CW, R>),
}

impl<'a, CW: Copy, R: AccessRules> Wraparoundable<'a, CW, R> {
    pub fn new(index: WraparoundableIndex, drawing: &'a Drawing<CW, R>) -> Self {
        match index {
            WraparoundableIndex::FixedDot(dot) => drawing.primitive(dot).into(),
            WraparoundableIndex::FixedBend(bend) => drawing.primitive(bend).into(),
            WraparoundableIndex::LooseBend(bend) => drawing.primitive(bend).into(),
        }
    }
}

impl<'a, CW: Copy, R: AccessRules> GetWraparound for FixedDot<'a, CW, R> {
    fn wraparound(&self) -> Option<LooseBendIndex> {
        self.first_rail()
    }
}

impl<'a, CW: Copy, R: AccessRules> GetWraparound for LooseBend<'a, CW, R> {
    fn wraparound(&self) -> Option<LooseBendIndex> {
        self.outer()
    }
}

impl<'a, CW: Copy, R: AccessRules> GetWraparound for FixedBend<'a, CW, R> {
    fn wraparound(&self) -> Option<LooseBendIndex> {
        self.first_rail()
    }
}
