use enum_dispatch::enum_dispatch;

use geo::{LineString, Point, Polygon};
use petgraph::stable_graph::NodeIndex;

use crate::{
    drawing::{
        dot::DotIndex,
        graph::{GetLayer, GetMaybeNet, MakePrimitive, PrimitiveIndex, PrimitiveWeight, Retag},
        primitive::{GenericPrimitive, Primitive},
        rules::RulesTrait,
        Drawing,
    },
    geometry::{poly::PolyShape, GetPos},
    graph::{GenericIndex, GetNodeIndex},
};

#[enum_dispatch]
pub trait MakePolyShape {
    fn shape<R: RulesTrait>(
        &self,
        drawing: &Drawing<ZoneWeight, R>,
        index: GenericIndex<ZoneWeight>,
    ) -> PolyShape;
}

#[enum_dispatch(GetLayer, GetMaybeNet, MakePolyShape)]
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

impl MakePolyShape for SolidZoneWeight {
    fn shape<R: RulesTrait>(
        &self,
        drawing: &Drawing<ZoneWeight, R>,
        index: GenericIndex<ZoneWeight>,
    ) -> PolyShape {
        PolyShape {
            polygon: Polygon::new(
                LineString::from(
                    drawing
                        .geometry()
                        .compound_members(index)
                        .filter_map(|primitive_node| {
                            if let Ok(dot) = DotIndex::try_from(primitive_node) {
                                Some(drawing.geometry().dot_weight(dot).pos())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<Point>>(),
                ),
                vec![],
            ),
        }
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

impl MakePolyShape for PourZoneWeight {
    fn shape<R: RulesTrait>(
        &self,
        drawing: &Drawing<ZoneWeight, R>,
        index: GenericIndex<ZoneWeight>,
    ) -> PolyShape {
        PolyShape {
            polygon: Polygon::new(
                LineString::from(
                    drawing
                        .geometry()
                        .compound_members(index)
                        .filter_map(|primitive_node| {
                            if let Ok(dot) = DotIndex::try_from(primitive_node) {
                                Some(drawing.geometry().dot_weight(dot).pos())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<Point>>(),
                ),
                vec![],
            ),
        }
    }
}

pub type PourZoneIndex = GenericIndex<PourZoneWeight>;
