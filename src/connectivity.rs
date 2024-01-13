use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::StableDiGraph;

use crate::{geometry::GetNet, graph::GenericIndex};

pub type ConnectivityGraph = StableDiGraph<ConnectivityWeight, ConnectivityLabel, usize>;

#[enum_dispatch(GetNet)]
#[derive(Debug, Clone, Copy)]
pub enum ConnectivityWeight {
    Component(ComponentWeight),
    Band(BandWeight),
}

#[derive(Debug, Clone, Copy)]
pub struct ComponentWeight {
    pub net: i64,
}

impl GetNet for ComponentWeight {
    fn net(&self) -> i64 {
        self.net
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BandWeight {
    pub net: i64,
    pub width: f64,
}

impl GetNet for BandWeight {
    fn net(&self) -> i64 {
        self.net
    }
}

pub type BandIndex = GenericIndex<BandWeight>;

#[enum_dispatch]
#[derive(Debug, Clone, Copy)]
pub enum ConnectivityLabel {}