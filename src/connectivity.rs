use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::StableDiGraph;

use crate::{geometry::FixedDotIndex, graph::GenericIndex};

#[enum_dispatch]
pub trait GetNet {
    fn net(&self) -> i64;
}

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

pub type ComponentIndex = GenericIndex<ComponentWeight>;

#[derive(Debug, Clone, Copy)]
pub struct BandWeight {
    pub net: i64,
    pub width: f64,
    pub from: FixedDotIndex,
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
