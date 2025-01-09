// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

//! Module for handling Vias properties

use serde::{Deserialize, Serialize};

use crate::{
    drawing::{
        graph::{GetMaybeNet, IsInLayer},
        primitive::MakePrimitiveShape,
        rules::AccessRules,
        Drawing,
    },
    geometry::primitive::{DotShape, PrimitiveShape},
    graph::{GenericIndex, GetPetgraphIndex},
    layout::CompoundWeight,
    math::Circle,
};

#[derive(Debug)]
pub struct Via<'a, R> {
    pub index: GenericIndex<ViaWeight>,
    drawing: &'a Drawing<CompoundWeight, R>,
}

impl<'a, R> Via<'a, R> {
    pub fn new(index: GenericIndex<ViaWeight>, drawing: &'a Drawing<CompoundWeight, R>) -> Self {
        Self { index, drawing }
    }
}

impl<'a, R: AccessRules> GetMaybeNet for Via<'a, R> {
    fn maybe_net(&self) -> Option<usize> {
        self.drawing.compound_weight(self.index.into()).maybe_net()
    }
}

impl<'a, R: AccessRules> MakePrimitiveShape for Via<'a, R> {
    fn shape(&self) -> PrimitiveShape {
        if let CompoundWeight::Via(weight) = self.drawing.compound_weight(self.index.into()) {
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

impl IsInLayer for ViaWeight {
    fn is_in_layer(&self, layer: usize) -> bool {
        (self.from_layer..=self.to_layer).contains(&layer)
    }

    fn is_in_any_layer_of(&self, layers: &[bool]) -> bool {
        layers
            .get(self.from_layer..=core::cmp::min(self.to_layer, layers.len()))
            .map(|i| i.iter().any(|j| *j))
            .unwrap_or(false)
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
