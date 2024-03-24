use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::StableDiGraph;

use crate::{
    drawing::{dot::FixedDotIndex, graph::GetMaybeNet, primitive::Primitive, rules::RulesTrait},
    graph::GenericIndex,
};

pub type ConnectivityGraph = StableDiGraph<ConnectivityWeight, ConnectivityLabel, usize>;

#[derive(Debug, Clone, Copy)]
pub enum ConnectivityWeight {
    Continent(ContinentWeight),
    Band(BandWeight),
}

#[derive(Debug, Clone, Copy)]
pub struct ContinentWeight {
    pub maybe_net: Option<usize>,
}

impl GetMaybeNet for ContinentWeight {
    fn maybe_net(&self) -> Option<usize> {
        self.maybe_net
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
