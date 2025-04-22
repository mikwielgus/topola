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
    graph::{GenericIndex, GetPetgraphIndex, MakeRef},
    layout::CompoundWeight,
};

use super::Layout;

#[enum_dispatch]
pub trait MakePolygon {
    fn shape(&self) -> Polygon;
}

#[derive(Debug)]
pub struct PolyRef<'a, R> {
    pub index: GenericIndex<PolyWeight>,
    drawing: &'a Drawing<CompoundWeight, R>,
}

impl<'a, R: AccessRules> MakeRef<'a, PolyRef<'a, R>, Layout<R>> for GenericIndex<PolyWeight> {
    fn ref_(&self, layout: &'a Layout<R>) -> PolyRef<'a, R> {
        PolyRef::new(*self, layout.drawing())
    }
}

pub(super) fn is_apex<'a, R: AccessRules>(
    drawing: &'a Drawing<CompoundWeight, R>,
    dot: FixedDotIndex,
) -> bool {
    !drawing
        .primitive(dot)
        .segs()
        .iter()
        .any(|seg| matches!(seg, SegIndex::Fixed(..)))
        && drawing.primitive(dot).bends().is_empty()
}

impl<'a, R: AccessRules> PolyRef<'a, R> {
    pub fn new(index: GenericIndex<PolyWeight>, drawing: &'a Drawing<CompoundWeight, R>) -> Self {
        Self { index, drawing }
    }

    fn is_apex(&self, dot: FixedDotIndex) -> bool {
        is_apex(self.drawing, dot)
    }

    pub fn apex(&self) -> FixedDotIndex {
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
            .unwrap()
    }
}

impl<R: AccessRules> GetLayer for PolyRef<'_, R> {
    fn layer(&self) -> usize {
        if let CompoundWeight::Poly(weight) = self.drawing.compound_weight(self.index.into()) {
            weight.layer()
        } else {
            unreachable!();
        }
    }
}

impl<R: AccessRules> GetMaybeNet for PolyRef<'_, R> {
    fn maybe_net(&self) -> Option<usize> {
        self.drawing.compound_weight(self.index.into()).maybe_net()
    }
}

impl<R: AccessRules> MakePolygon for PolyRef<'_, R> {
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
