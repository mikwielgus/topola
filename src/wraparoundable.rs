use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;

use crate::{
    drawing::{
        bend::{BendIndex, FixedBendIndex, LooseBendIndex},
        dot::FixedDotIndex,
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::{FixedBend, FixedDot, GetFirstRail, GetInnerOuter, LooseBend, Primitive},
        rules::RulesTrait,
        Drawing,
    },
    graph::GetNodeIndex,
};

#[enum_dispatch]
pub trait GetWraparound: GetNodeIndex {
    fn wraparound(&self) -> Option<LooseBendIndex>;
}

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
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

#[enum_dispatch(GetWraparound, GetDrawing, GetNodeIndex)]
pub enum Wraparoundable<'a, GW: Copy, R: RulesTrait> {
    FixedDot(FixedDot<'a, GW, R>),
    FixedBend(FixedBend<'a, GW, R>),
    LooseBend(LooseBend<'a, GW, R>),
}

impl<'a, GW: Copy, R: RulesTrait> Wraparoundable<'a, GW, R> {
    pub fn new(index: WraparoundableIndex, drawing: &'a Drawing<GW, R>) -> Self {
        match index {
            WraparoundableIndex::FixedDot(dot) => drawing.primitive(dot).into(),
            WraparoundableIndex::FixedBend(bend) => drawing.primitive(bend).into(),
            WraparoundableIndex::LooseBend(bend) => drawing.primitive(bend).into(),
        }
    }
}

impl<'a, GW: Copy, R: RulesTrait> GetWraparound for FixedDot<'a, GW, R> {
    fn wraparound(&self) -> Option<LooseBendIndex> {
        self.first_rail()
    }
}

impl<'a, GW: Copy, R: RulesTrait> GetWraparound for LooseBend<'a, GW, R> {
    fn wraparound(&self) -> Option<LooseBendIndex> {
        self.outer()
    }
}

impl<'a, GW: Copy, R: RulesTrait> GetWraparound for FixedBend<'a, GW, R> {
    fn wraparound(&self) -> Option<LooseBendIndex> {
        self.first_rail()
    }
}
