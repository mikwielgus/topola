use enum_dispatch::enum_dispatch;

use petgraph::stable_graph::NodeIndex;

use crate::{
    drawing::{
        graph::{GeometryIndex, GeometryWeight, GetLayer, GetMaybeNet, MakePrimitive, Retag},
        primitive::{GenericPrimitive, Primitive},
        rules::RulesTrait,
        Drawing,
    },
    graph::{GenericIndex, GetNodeIndex},
};

#[enum_dispatch(GetNodeIndex)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GroupingIndex {
    Solid(SolidGroupingIndex),
    Pour(PourGroupingIndex),
}

#[enum_dispatch(GetLayer)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GroupingWeight {
    Solid(SolidGroupingWeight),
    Pour(PourGroupingWeight),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolidGroupingWeight {
    pub layer: u64,
    pub maybe_net: Option<usize>,
}

impl<'a> GetLayer for SolidGroupingWeight {
    fn layer(&self) -> u64 {
        self.layer
    }
}

impl<'a> GetMaybeNet for SolidGroupingWeight {
    fn maybe_net(&self) -> Option<usize> {
        self.maybe_net
    }
}

pub type SolidGroupingIndex = GenericIndex<SolidGroupingWeight>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PourGroupingWeight {
    pub layer: u64,
    pub maybe_net: Option<usize>,
}

impl<'a> GetLayer for PourGroupingWeight {
    fn layer(&self) -> u64 {
        self.layer
    }
}

impl<'a> GetMaybeNet for PourGroupingWeight {
    fn maybe_net(&self) -> Option<usize> {
        self.maybe_net
    }
}

pub type PourGroupingIndex = GenericIndex<PourGroupingWeight>;
