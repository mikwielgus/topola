use crate::{
    drawing::{graph::GetMaybeNet, rules::RulesTrait},
    geometry::compound::CompoundManagerTrait,
    graph::{GenericIndex, GetNodeIndex},
    layout::{CompoundWeight, Layout},
    math::Circle,
};

#[derive(Debug)]
pub struct Via<'a, R: RulesTrait> {
    pub index: GenericIndex<ViaWeight>,
    layout: &'a Layout<R>,
}

impl<'a, R: RulesTrait> Via<'a, R> {
    pub fn new(index: GenericIndex<ViaWeight>, layout: &'a Layout<R>) -> Self {
        Self { index, layout }
    }
}

impl<'a, R: RulesTrait> GetMaybeNet for Via<'a, R> {
    fn maybe_net(&self) -> Option<usize> {
        self.layout
            .drawing()
            .compound_weight(self.index.into())
            .maybe_net()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ViaWeight {
    pub from_layer: u64,
    pub to_layer: u64,
    pub circle: Circle,
    pub maybe_net: Option<usize>,
}

impl GetMaybeNet for ViaWeight {
    fn maybe_net(&self) -> Option<usize> {
        self.maybe_net
    }
}

impl From<GenericIndex<ViaWeight>> for GenericIndex<CompoundWeight> {
    fn from(via: GenericIndex<ViaWeight>) -> Self {
        GenericIndex::<CompoundWeight>::new(via.node_index())
    }
}
