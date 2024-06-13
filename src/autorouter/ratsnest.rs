use std::collections::HashMap;

use enum_dispatch::enum_dispatch;
use geo::Point;
use petgraph::{
    data::Element,
    graph::{EdgeIndex, NodeIndex, UnGraph},
    unionfind::UnionFind,
    visit::{EdgeRef, IntoEdgeReferences, NodeIndexable},
};
use spade::{HasPosition, InsertionError, Point2};

use crate::{
    drawing::{
        band::BandIndex,
        dot::FixedDotIndex,
        graph::{GetMaybeNet, MakePrimitive, PrimitiveIndex},
        primitive::MakePrimitiveShape,
        rules::RulesTrait,
    },
    geometry::{compound::CompoundManagerTrait, shape::ShapeTrait},
    graph::{GenericIndex, GetNodeIndex},
    layout::{
        zone::{MakePolyShape, ZoneWeight},
        Layout,
    },
    triangulation::{GetTrianvertexIndex, Triangulation},
};

#[enum_dispatch(GetNodeIndex)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RatvertexIndex {
    FixedDot(FixedDotIndex),
    Zone(GenericIndex<ZoneWeight>),
}

impl From<RatvertexIndex> for crate::layout::NodeIndex {
    fn from(vertex: RatvertexIndex) -> crate::layout::NodeIndex {
        match vertex {
            RatvertexIndex::FixedDot(dot) => crate::layout::NodeIndex::Primitive(dot.into()),
            RatvertexIndex::Zone(zone) => crate::layout::NodeIndex::Compound(zone.into()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RatvertexWeight {
    vertex: RatvertexIndex,
    pub pos: Point,
}

impl GetTrianvertexIndex<RatvertexIndex> for RatvertexWeight {
    fn trianvertex_index(&self) -> RatvertexIndex {
        self.vertex
    }
}

impl HasPosition for RatvertexWeight {
    type Scalar = f64;
    fn position(&self) -> Point2<Self::Scalar> {
        Point2::new(self.pos.x(), self.pos.y())
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RatlineWeight {
    pub band: Option<BandIndex>,
}

pub struct Ratsnest {
    graph: UnGraph<RatvertexWeight, RatlineWeight, usize>,
}

impl Ratsnest {
    pub fn new(layout: &Layout<impl RulesTrait>) -> Result<Self, InsertionError> {
        let mut unionfind = UnionFind::new(layout.drawing().geometry().graph().node_bound());

        for edge in layout.drawing().geometry().graph().edge_references() {
            unionfind.union(edge.source(), edge.target());
        }

        let mut this = Self {
            graph: UnGraph::default(),
        };

        let mut triangulations = HashMap::new();

        for layer in 0..layout.drawing().layer_count() {
            for node in layout.drawing().layer_primitive_nodes(layer) {
                match node {
                    PrimitiveIndex::FixedDot(dot) => {
                        if layout.zones(dot).next().is_none() {
                            if let Some(net) = layout.drawing().primitive(dot).maybe_net() {
                                if !triangulations.contains_key(&(layer, net)) {
                                    triangulations.insert(
                                        (layer, net),
                                        Triangulation::new(
                                            layout.drawing().geometry().graph().node_bound(),
                                        ),
                                    );
                                }

                                triangulations.get_mut(&(layer, net)).unwrap().add_vertex(
                                    RatvertexWeight {
                                        vertex: RatvertexIndex::FixedDot(dot),
                                        pos: node.primitive(layout.drawing()).shape().center(),
                                    },
                                )?;
                            }
                        }
                    }
                    _ => (),
                }
            }

            for zone in layout.layer_zone_nodes(layer) {
                if let Some(net) = layout.drawing().compound_weight(zone.into()).maybe_net() {
                    if !triangulations.contains_key(&(layer, net)) {
                        triangulations.insert(
                            (layer, net),
                            Triangulation::new(layout.drawing().geometry().graph().node_bound()),
                        );
                    }

                    triangulations
                        .get_mut(&(layer, net))
                        .unwrap()
                        .add_vertex(RatvertexWeight {
                            vertex: RatvertexIndex::Zone(zone),
                            pos: layout.zone(zone).shape().center(),
                        })?
                }
            }
        }

        for ((_layer, _net), triangulation) in triangulations {
            let mut map = Vec::new();

            for element in petgraph::algo::min_spanning_tree(&triangulation) {
                match element {
                    Element::Node { weight } => {
                        map.push(this.graph.add_node(weight));
                    }
                    Element::Edge {
                        source,
                        target,
                        weight,
                    } => {
                        this.graph.add_edge(map[source], map[target], weight.weight);
                    }
                }
            }
        }

        this.graph.retain_edges(|g, i| {
            if let Some((source, target)) = g.edge_endpoints(i) {
                let source_index = g
                    .node_weight(source)
                    .unwrap()
                    .trianvertex_index()
                    .node_index();
                let target_index = g
                    .node_weight(target)
                    .unwrap()
                    .trianvertex_index()
                    .node_index();
                !unionfind.equiv(source_index, target_index)
            } else {
                true
            }
        });

        Ok(this)
    }

    pub fn assign_band_to_ratline(&mut self, ratline: EdgeIndex<usize>, band: BandIndex) {
        self.graph.edge_weight_mut(ratline).unwrap().band = Some(band);
    }

    pub fn graph(&self) -> &UnGraph<RatvertexWeight, RatlineWeight, usize> {
        &self.graph
    }
}
