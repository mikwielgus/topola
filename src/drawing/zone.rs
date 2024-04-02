use enum_dispatch::enum_dispatch;

use petgraph::stable_graph::NodeIndex;

use crate::{
    drawing::{
        graph::{GetLayer, GetMaybeNet, MakePrimitive, PrimitiveIndex, PrimitiveWeight, Retag},
        primitive::{GenericPrimitive, Primitive},
        rules::RulesTrait,
        Drawing,
    },
    graph::{GenericIndex, GetNodeIndex},
};

#[enum_dispatch(GetNodeIndex)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZoneIndex {
    Solid(SolidZoneIndex),
    Pour(PourZoneIndex),
}

#[enum_dispatch(GetLayer)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZoneWeight {
    Solid(SolidZoneWeight),
    Pour(PourZoneWeight),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolidZoneWeight {
    pub layer: u64,
    pub maybe_net: Option<usize>,
}

impl<'a> GetLayer for SolidZoneWeight {
    fn layer(&self) -> u64 {
        self.layer
    }
}

impl<'a> GetMaybeNet for SolidZoneWeight {
    fn maybe_net(&self) -> Option<usize> {
        self.maybe_net
    }
}

pub type SolidZoneIndex = GenericIndex<SolidZoneWeight>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PourZoneWeight {
    pub layer: u64,
    pub maybe_net: Option<usize>,
}

impl<'a> GetLayer for PourZoneWeight {
    fn layer(&self) -> u64 {
        self.layer
    }
}

impl<'a> GetMaybeNet for PourZoneWeight {
    fn maybe_net(&self) -> Option<usize> {
        self.maybe_net
    }
}

pub type PourZoneIndex = GenericIndex<PourZoneWeight>;
