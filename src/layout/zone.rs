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
    geometry::GetPos,
    graph::{GenericIndex, GetNodeIndex},
};

#[enum_dispatch]
pub trait MakePolygon {
    fn polygon<R: RulesTrait>(&self, drawing: &Drawing<impl Copy, R>) -> Polygon;
}

#[enum_dispatch(GetNodeIndex, MakePolygon)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZoneIndex {
    Solid(SolidZoneIndex),
    Pour(PourZoneIndex),
}

#[enum_dispatch(GetLayer)]
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

impl MakePolygon for SolidZoneIndex {
    fn polygon<R: RulesTrait>(&self, drawing: &Drawing<impl Copy, R>) -> Polygon {
        Polygon::new(
            LineString::from(
                drawing
                    .geometry()
                    .grouping_members(GenericIndex::new(self.node_index()))
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
        )
    }
}

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

impl MakePolygon for PourZoneIndex {
    fn polygon<R: RulesTrait>(&self, drawing: &Drawing<impl Copy, R>) -> Polygon {
        Polygon::new(
            LineString::from(
                drawing
                    .geometry()
                    .grouping_members(GenericIndex::new(self.node_index()))
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
        )
    }
}
