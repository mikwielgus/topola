use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::StableDiGraph;

use crate::{graph::GenericIndex, layout::dot::FixedDotIndex};

#[enum_dispatch]
pub trait GetNet {
    fn net(&self) -> i64;
}

pub type ConnectivityGraph = StableDiGraph<ConnectivityWeight, ConnectivityLabel, usize>;

#[enum_dispatch(GetNet)]
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

#[derive(Debug, Clone, Copy)]
pub enum ConnectivityLabel {
    Band,
}
