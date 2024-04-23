use enum_dispatch::enum_dispatch;
use geo::Point;
use petgraph::{
    data::FromElements,
    stable_graph::{NodeIndex, StableUnGraph},
    unionfind::UnionFind,
    visit::{self, EdgeRef, IntoEdgeReferences, NodeIndexable},
};
use spade::{HasPosition, InsertionError, Point2};

use crate::{
    drawing::{
        dot::FixedDotIndex,
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::MakePrimitiveShape,
        rules::RulesTrait,
    },
    geometry::{compound::CompoundManagerTrait, shape::ShapeTrait},
    graph::{GenericIndex, GetNodeIndex},
    layout::{
        zone::{MakePolyShape, ZoneWeight},
        Layout,
    },
    triangulation::{GetVertexIndex, Triangulation, TriangulationEdgeWeight},
};

#[enum_dispatch(GetNodeIndex)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RatsnestVertexIndex {
    FixedDot(FixedDotIndex),
    Zone(GenericIndex<ZoneWeight>),
}

#[derive(Debug, Clone, Copy)]
pub struct VertexWeight {
    vertex: RatsnestVertexIndex,
    pub pos: Point,
}

impl GetVertexIndex<RatsnestVertexIndex> for VertexWeight {
    fn vertex_index(&self) -> RatsnestVertexIndex {
        self.vertex
    }
}

impl HasPosition for VertexWeight {
    type Scalar = f64;
    fn position(&self) -> Point2<Self::Scalar> {
        Point2::new(self.pos.x(), self.pos.y())
    }
}

pub struct Ratsnest {
    graph: StableUnGraph<VertexWeight, TriangulationEdgeWeight, usize>,
}

impl Ratsnest {
    pub fn new(layout: &Layout<impl RulesTrait>) -> Result<Self, InsertionError> {
        let mut this = Self {
            graph: StableUnGraph::default(),
        };

        let mut triangulation =
            Triangulation::new(layout.drawing().geometry().graph().node_bound());

        for node in layout.drawing().primitive_nodes() {
            match node {
                PrimitiveIndex::FixedDot(dot) => {
                    if layout.compounds(dot).next().is_none() {
                        triangulation.add_vertex(VertexWeight {
                            vertex: RatsnestVertexIndex::FixedDot(dot),
                            pos: node.primitive(layout.drawing()).shape().center(),
                        })?;
                    }
                }
                _ => (),
            }
        }

        for zone in layout.zones() {
            triangulation.add_vertex(VertexWeight {
                vertex: RatsnestVertexIndex::Zone(zone),
                pos: layout
                    .compound_weight(zone)
                    .shape(&layout.drawing(), zone)
                    .center(),
            })?
        }

        this.graph =
            StableUnGraph::from_elements(petgraph::algo::min_spanning_tree(&triangulation));

        let mut unionfind = UnionFind::new(layout.drawing().geometry().graph().node_bound());

        for edge in layout.drawing().geometry().graph().edge_references() {
            unionfind.union(edge.source(), edge.target());
        }

        this.graph.retain_edges(|g, i| {
            if let Some((from, to)) = g.edge_endpoints(i) {
                let from_index = g.node_weight(from).unwrap().vertex_index().node_index();
                let to_index = g.node_weight(to).unwrap().vertex_index().node_index();
                !unionfind.equiv(from_index, to_index)
            } else {
                true
            }
        });

        Ok(this)
    }

    pub fn graph(&self) -> &StableUnGraph<VertexWeight, TriangulationEdgeWeight, usize> {
        &self.graph
    }
}
