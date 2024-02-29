use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::StableDiGraph;

use crate::{
    graph::GenericIndex,
    layout::{dot::FixedDotIndex, graph::GetNet, primitive::Primitive, rules::RulesTrait},
};

pub type ConnectivityGraph = StableDiGraph<ConnectivityWeight, ConnectivityLabel, usize>;

#[derive(Debug, Clone, Copy)]
pub enum ConnectivityWeight {
    Continent(ContinentWeight),
    Band(BandWeight),
}

#[derive(Debug, Clone, Copy)]
pub struct ContinentWeight {
    pub net: i64,
}

impl GetNet for ContinentWeight {
    fn net(&self) -> i64 {
        self.net
    }
}

pub type ContinentIndex = GenericIndex<ContinentWeight>;

#[derive(Debug, Clone, Copy)]
pub struct BandWeight {
    pub from: FixedDotIndex,
    pub to: Option<FixedDotIndex>,
}

pub type BandIndex = GenericIndex<BandWeight>;

#[derive(Debug, Clone, Copy)]
pub enum ConnectivityLabel {
    Band,
}
