use enum_dispatch::enum_dispatch;

use geo::{LineString, Point, Polygon};

use crate::{
    drawing::{
        dot::FixedDotIndex,
        graph::{GetLayer, GetMaybeNet, PrimitiveIndex},
        primitive::GetLimbs,
        rules::AccessRules,
        seg::SegIndex,
    },
    geometry::{compound::ManageCompounds, poly::PolyShape, GetPos},
    graph::{GenericIndex, GetPetgraphIndex},
    layout::{CompoundWeight, Layout},
};

#[enum_dispatch]
pub trait MakePolyShape {
    fn shape(&self) -> PolyShape;
}

#[enum_dispatch]
pub trait GetMaybeApex {
    fn maybe_apex(&self) -> Option<FixedDotIndex>;
}

#[derive(Debug)]
pub struct Poly<'a, R: AccessRules> {
    pub index: GenericIndex<PolyWeight>,
    layout: &'a Layout<R>,
}

impl<'a, R: AccessRules> Poly<'a, R> {
    pub fn new(index: GenericIndex<PolyWeight>, layout: &'a Layout<R>) -> Self {
        Self { index, layout }
    }

    fn is_apex(&self, dot: FixedDotIndex) -> bool {
        !self
            .layout
            .drawing()
            .primitive(dot)
            .segs()
            .iter()
            .any(|seg| matches!(seg, SegIndex::Fixed(..)))
            && self.layout.drawing().primitive(dot).bends().is_empty()
    }
}

impl<'a, R: AccessRules> GetLayer for Poly<'a, R> {
    fn layer(&self) -> usize {
        if let CompoundWeight::Poly(weight) =
            self.layout.drawing().compound_weight(self.index.into())
        {
            weight.layer()
        } else {
            unreachable!();
        }
    }
}

impl<'a, R: AccessRules> GetMaybeNet for Poly<'a, R> {
    fn maybe_net(&self) -> Option<usize> {
        self.layout
            .drawing()
            .compound_weight(self.index.into())
            .maybe_net()
    }
}

impl<'a, R: AccessRules> MakePolyShape for Poly<'a, R> {
    fn shape(&self) -> PolyShape {
        PolyShape {
            polygon: Polygon::new(
                LineString::from(
                    self.layout
                        .drawing()
                        .geometry()
                        .compound_members(self.index.into())
                        .filter_map(|primitive_node| {
                            let PrimitiveIndex::FixedDot(dot) = primitive_node else {
                                return None;
                            };

                            if self.is_apex(dot) {
                                None
                            } else {
                                Some(
                                    self.layout
                                        .drawing()
                                        .geometry()
                                        .dot_weight(dot.into())
                                        .pos(),
                                )
                            }
                        })
                        .collect::<Vec<Point>>(),
                ),
                vec![],
            ),
        }
    }
}

impl<'a, R: AccessRules> GetMaybeApex for Poly<'a, R> {
    fn maybe_apex(&self) -> Option<FixedDotIndex> {
        self.layout
            .drawing()
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
