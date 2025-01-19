// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

//! Module for handling Polygon properties

use enum_dispatch::enum_dispatch;

use geo::{LineString, Point, Polygon};

use crate::{
    drawing::{
        dot::FixedDotIndex,
        graph::{GetMaybeNet, PrimitiveIndex},
        primitive::GetLimbs,
        rules::AccessRules,
        seg::SegIndex,
        Drawing,
    },
    geometry::{GetLayer, GetSetPos},
    graph::{GenericIndex, GetPetgraphIndex},
    layout::CompoundWeight,
};

#[enum_dispatch]
pub trait MakePolygon {
    fn shape(&self) -> Polygon;
}

#[enum_dispatch]
pub trait GetMaybeApex {
    fn maybe_apex(&self) -> Option<FixedDotIndex>;
}

#[derive(Debug)]
pub struct Poly<'a, R> {
    pub index: GenericIndex<PolyWeight>,
    drawing: &'a Drawing<CompoundWeight, R>,
}

impl<'a, R: AccessRules> Poly<'a, R> {
    pub fn new(index: GenericIndex<PolyWeight>, drawing: &'a Drawing<CompoundWeight, R>) -> Self {
        Self { index, drawing }
    }

    fn is_apex(&self, dot: FixedDotIndex) -> bool {
        !self
            .drawing
            .primitive(dot)
            .segs()
            .iter()
            .any(|seg| matches!(seg, SegIndex::Fixed(..)))
            && self.drawing.primitive(dot).bends().is_empty()
    }
}

impl<'a, R: AccessRules> GetLayer for Poly<'a, R> {
    fn layer(&self) -> usize {
        if let CompoundWeight::Poly(weight) = self.drawing.compound_weight(self.index.into()) {
            weight.layer()
        } else {
            unreachable!();
        }
    }
}

impl<'a, R: AccessRules> GetMaybeNet for Poly<'a, R> {
    fn maybe_net(&self) -> Option<usize> {
        self.drawing.compound_weight(self.index.into()).maybe_net()
    }
}

impl<'a, R: AccessRules> MakePolygon for Poly<'a, R> {
    fn shape(&self) -> Polygon {
        Polygon::new(
            LineString::from(
                self.drawing
                    .geometry()
                    .compound_members(self.index.into())
                    .filter_map(|primitive_node| {
                        let PrimitiveIndex::FixedDot(dot) = primitive_node else {
                            return None;
                        };

                        if self.is_apex(dot) {
                            None
                        } else {
                            Some(self.drawing.geometry().dot_weight(dot.into()).pos())
                        }
                    })
                    .collect::<Vec<Point>>(),
            ),
            vec![],
        )
    }
}

impl<'a, R: AccessRules> GetMaybeApex for Poly<'a, R> {
    fn maybe_apex(&self) -> Option<FixedDotIndex> {
        self.drawing
            .geometry()
            .compound_members(self.index.into())
            .find_map(|primitive_node| {
                if let PrimitiveIndex::FixedDot(dot) = primitive_node {
                    if self.is_apex(dot) {
                        return Some(dot);
                    }
                }

                None
            })
    }
}

#[enum_dispatch(GetLayer, GetMaybeNet)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PolyWeight {
    Solid(SolidPolyWeight),
    Pour(PourPolyWeight),
}

impl From<GenericIndex<PolyWeight>> for GenericIndex<CompoundWeight> {
    fn from(poly: GenericIndex<PolyWeight>) -> Self {
        GenericIndex::<CompoundWeight>::new(poly.petgraph_index())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolidPolyWeight {
    pub layer: usize,
    pub maybe_net: Option<usize>,
}

impl GetLayer for SolidPolyWeight {
    fn layer(&self) -> usize {
        self.layer
    }
}

impl GetMaybeNet for SolidPolyWeight {
    fn maybe_net(&self) -> Option<usize> {
        self.maybe_net
    }
}

impl From<GenericIndex<SolidPolyWeight>> for GenericIndex<CompoundWeight> {
    fn from(poly: GenericIndex<SolidPolyWeight>) -> Self {
        GenericIndex::<CompoundWeight>::new(poly.petgraph_index())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PourPolyWeight {
    pub layer: usize,
    pub maybe_net: Option<usize>,
}

impl GetLayer for PourPolyWeight {
    fn layer(&self) -> usize {
        self.layer
    }
}

impl GetMaybeNet for PourPolyWeight {
    fn maybe_net(&self) -> Option<usize> {
        self.maybe_net
    }
}

impl From<GenericIndex<PourPolyWeight>> for GenericIndex<CompoundWeight> {
    fn from(poly: GenericIndex<PourPolyWeight>) -> Self {
        GenericIndex::<CompoundWeight>::new(poly.petgraph_index())
    }
}
