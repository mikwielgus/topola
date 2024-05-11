use enum_dispatch::enum_dispatch;

use geo::{LineString, Point, Polygon};
use petgraph::stable_graph::NodeIndex;

use crate::{
    drawing::{
        dot::{DotIndex, FixedDotIndex},
        graph::{GetLayer, GetMaybeNet, MakePrimitive, PrimitiveIndex, PrimitiveWeight, Retag},
        primitive::{GenericPrimitive, GetLimbs, Primitive},
        rules::RulesTrait,
        seg::SegIndex,
        Drawing,
    },
    geometry::{compound::CompoundManagerTrait, poly::PolyShape, GetPos},
    graph::{GenericIndex, GetNodeIndex},
    layout::Layout,
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
pub struct Zone<'a, R: RulesTrait> {
    pub index: GenericIndex<ZoneWeight>,
    layout: &'a Layout<R>,
}

impl<'a, R: RulesTrait> Zone<'a, R> {
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

impl<'a, R: RulesTrait> GetLayer for Zone<'a, R> {
    fn layer(&self) -> u64 {
        self.layout.drawing().compound_weight(self.index).layer()
    }
}

impl<'a, R: RulesTrait> GetMaybeNet for Zone<'a, R> {
    fn maybe_net(&self) -> Option<usize> {
        self.layout
            .drawing()
            .compound_weight(self.index)
            .maybe_net()
    }
}

impl<'a, R: RulesTrait> MakePolyShape for Zone<'a, R> {
    fn shape(&self) -> PolyShape {
        PolyShape {
            polygon: Polygon::new(
                LineString::from(
                    self.layout
                        .drawing()
                        .geometry()
                        .compound_members(self.index)
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

impl<'a, R: RulesTrait> GetMaybeApex for Zone<'a, R> {
    fn maybe_apex(&self) -> Option<FixedDotIndex> {
        self.layout
            .drawing()
            .geometry()
            .compound_members(self.index)
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolidZoneWeight {
    pub layer: u64,
    pub maybe_net: Option<usize>,
}

impl<'a> GetLayer for SolidZoneWeight {
    fn layer(&self) -> u64 {
        self.layer
    }
}

impl<'a> GetMaybeNet for SolidZoneWeight {
    fn maybe_net(&self) -> Option<usize> {
        self.maybe_net
    }
}

pub type SolidZoneIndex = GenericIndex<SolidZoneWeight>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PourZoneWeight {
    pub layer: u64,
    pub maybe_net: Option<usize>,
}

impl<'a> GetLayer for PourZoneWeight {
    fn layer(&self) -> u64 {
        self.layer
    }
}

impl<'a> GetMaybeNet for PourZoneWeight {
    fn maybe_net(&self) -> Option<usize> {
        self.maybe_net
    }
}

pub type PourZoneIndex = GenericIndex<PourZoneWeight>;
