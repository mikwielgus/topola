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

#[enum_dispatch(GetWraparound, GetLayout, GetNodeIndex)]
pub enum Wraparoundable<'a, R: RulesTrait> {
    FixedDot(FixedDot<'a, R>),
    FixedBend(FixedBend<'a, R>),
    LooseBend(LooseBend<'a, R>),
}

impl<'a, R: RulesTrait> Wraparoundable<'a, R> {
    pub fn new(index: WraparoundableIndex, drawing: &'a Drawing<R>) -> Self {
        match index {
            WraparoundableIndex::FixedDot(dot) => drawing.primitive(dot).into(),
            WraparoundableIndex::FixedBend(bend) => drawing.primitive(bend).into(),
            WraparoundableIndex::LooseBend(bend) => drawing.primitive(bend).into(),
        }
    }
}

impl<'a, R: RulesTrait> GetWraparound for FixedDot<'a, R> {
    fn wraparound(&self) -> Option<LooseBendIndex> {
        self.first_rail()
    }
}

impl<'a, R: RulesTrait> GetWraparound for LooseBend<'a, R> {
    fn wraparound(&self) -> Option<LooseBendIndex> {
        self.outer()
    }
}

impl<'a, R: RulesTrait> GetWraparound for FixedBend<'a, R> {
    fn wraparound(&self) -> Option<LooseBendIndex> {
        self.first_rail()
    }
}
