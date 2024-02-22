use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;

use crate::{
    graph::GetNodeIndex,
    layout::{
        bend::{BendIndex, FixedBendIndex, LooseBendIndex},
        dot::FixedDotIndex,
        graph::{GeometryIndex, MakePrimitive},
        primitive::{FixedBend, FixedDot, GetFirstRail, GetInnerOuter, LooseBend, Primitive},
        rules::RulesTrait,
        Layout,
    },
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

impl From<WraparoundableIndex> for GeometryIndex {
    fn from(wraparoundable: WraparoundableIndex) -> Self {
        match wraparoundable {
            WraparoundableIndex::FixedDot(dot) => GeometryIndex::FixedDot(dot),
            WraparoundableIndex::FixedBend(bend) => GeometryIndex::FixedBend(bend),
            WraparoundableIndex::LooseBend(bend) => GeometryIndex::LooseBend(bend),
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
    pub fn new(index: WraparoundableIndex, layout: &'a Layout<R>) -> Self {
        match index {
            WraparoundableIndex::FixedDot(dot) => layout.primitive(dot).into(),
            WraparoundableIndex::FixedBend(bend) => layout.primitive(bend).into(),
            WraparoundableIndex::LooseBend(bend) => layout.primitive(bend).into(),
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
