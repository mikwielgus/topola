use enum_dispatch::enum_dispatch;

use geo::{LineString, Point, Polygon};

use crate::{
    drawing::{
        dot::FixedDotIndex,
        graph::{GetLayer, GetMaybeNet, MakePrimitive, PrimitiveIndex},
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
pub struct Zone<'a, R: AccessRules> {
    pub index: GenericIndex<ZoneWeight>,
    layout: &'a Layout<R>,
}

impl<'a, R: AccessRules> Zone<'a, R> {
    pub fn new(index: GenericIndex<ZoneWeight>, layout: &'a Layout<R>) -> Self {
        Self { index, layout }
    }

    fn is_apex(&self, dot: FixedDotIndex) -> bool {
        self.layout
            .drawing()
            .primitive(dot)
            .segs()
            .iter()
            .find(|seg| matches!(seg, SegIndex::Fixed(..)))
            .is_none()
            && self.layout.drawing().primitive(dot).bends().is_empty()
    }
}

impl<'a, R: AccessRules> GetLayer for Zone<'a, R> {
    fn layer(&self) -> usize {
        if let CompoundWeight::Zone(weight) =
            self.layout.drawing().compound_weight(self.index.into())
        {
            weight.layer()
        } else {
            unreachable!();
        }
    }
}

impl<'a, R: AccessRules> GetMaybeNet for Zone<'a, R> {
    fn maybe_net(&self) -> Option<usize> {
        self.layout
            .drawing()
            .compound_weight(self.index.into())
            .maybe_net()
    }
}

impl<'a, R: AccessRules> MakePolyShape for Zone<'a, R> {
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
                                return None;
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

impl<'a, R: AccessRules> GetMaybeApex for Zone<'a, R> {
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
pub enum ZoneWeight {
    Solid(SolidZoneWeight),
    Pour(PourZoneWeight),
}

impl From<GenericIndex<ZoneWeight>> for GenericIndex<CompoundWeight> {
    fn from(zone: GenericIndex<ZoneWeight>) -> Self {
        GenericIndex::<CompoundWeight>::new(zone.petgraph_index())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolidZoneWeight {
    pub layer: usize,
    pub maybe_net: Option<usize>,
}

impl GetLayer for SolidZoneWeight {
    fn layer(&self) -> usize {
        self.layer
    }
}

impl GetMaybeNet for SolidZoneWeight {
    fn maybe_net(&self) -> Option<usize> {
        self.maybe_net
    }
}

impl From<GenericIndex<SolidZoneWeight>> for GenericIndex<CompoundWeight> {
    fn from(zone: GenericIndex<SolidZoneWeight>) -> Self {
        GenericIndex::<CompoundWeight>::new(zone.petgraph_index())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PourZoneWeight {
    pub layer: usize,
    pub maybe_net: Option<usize>,
}

impl<'a> GetLayer for PourZoneWeight {
    fn layer(&self) -> usize {
        self.layer
    }
}

impl<'a> GetMaybeNet for PourZoneWeight {
    fn maybe_net(&self) -> Option<usize> {
        self.maybe_net
    }
}

impl From<GenericIndex<PourZoneWeight>> for GenericIndex<CompoundWeight> {
    fn from(zone: GenericIndex<PourZoneWeight>) -> Self {
        GenericIndex::<CompoundWeight>::new(zone.petgraph_index())
    }
}
