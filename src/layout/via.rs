use serde::{Deserialize, Serialize};

use crate::{
    drawing::{graph::GetMaybeNet, primitive::MakePrimitiveShape, rules::RulesTrait},
    geometry::{
        compound::CompoundManagerTrait,
        primitive::{DotShape, PrimitiveShape},
    },
    graph::{GenericIndex, GetPetgraphIndex},
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

impl<'a, R: RulesTrait> MakePrimitiveShape for Via<'a, R> {
    fn shape(&self) -> PrimitiveShape {
        if let CompoundWeight::Via(weight) =
            self.layout.drawing().compound_weight(self.index.into())
        {
            weight.shape()
        } else {
            unreachable!();
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ViaWeight {
    pub from_layer: usize,
    pub to_layer: usize,
    pub circle: Circle,
    pub maybe_net: Option<usize>,
}

impl From<GenericIndex<ViaWeight>> for GenericIndex<CompoundWeight> {
    fn from(via: GenericIndex<ViaWeight>) -> Self {
        GenericIndex::<CompoundWeight>::new(via.petgraph_index())
    }
}

impl GetMaybeNet for ViaWeight {
    fn maybe_net(&self) -> Option<usize> {
        self.maybe_net
    }
}

impl MakePrimitiveShape for ViaWeight {
    fn shape(&self) -> PrimitiveShape {
        DotShape {
            circle: self.circle,
        }
        .into()
    }
}
