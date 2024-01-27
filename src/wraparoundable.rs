use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;

use crate::{
    graph::GetNodeIndex,
    layout::Layout,
    layout::{
        bend::{BendIndex, FixedBendIndex, LooseBendIndex},
        dot::FixedDotIndex,
        geometry::{GeometryIndex, MakePrimitive},
    },
    primitive::{
        FixedBend, FixedDot, GetFirstRail, GetInnerOuter, GetLayout, LooseBend, Primitive,
    },
};

#[enum_dispatch]
pub trait GetWraparound: GetLayout + GetNodeIndex {
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
pub enum Wraparoundable<'a> {
    FixedDot(FixedDot<'a>),
    FixedBend(FixedBend<'a>),
    LooseBend(LooseBend<'a>),
}

impl<'a> Wraparoundable<'a> {
    pub fn new(index: WraparoundableIndex, layout: &'a Layout) -> Self {
        match index {
            WraparoundableIndex::FixedDot(dot) => layout.primitive(dot).into(),
            WraparoundableIndex::FixedBend(bend) => layout.primitive(bend).into(),
            WraparoundableIndex::LooseBend(bend) => layout.primitive(bend).into(),
        }
    }
}

impl<'a> GetWraparound for FixedDot<'a> {
    fn wraparound(&self) -> Option<LooseBendIndex> {
        self.first_rail()
    }
}

impl<'a> GetWraparound for LooseBend<'a> {
    fn wraparound(&self) -> Option<LooseBendIndex> {
        self.outer()
    }
}

impl<'a> GetWraparound for FixedBend<'a> {
    fn wraparound(&self) -> Option<LooseBendIndex> {
        self.first_rail()
    }
}
