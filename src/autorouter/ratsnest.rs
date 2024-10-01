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
        band::BandTermsegIndex,
        dot::FixedDotIndex,
        graph::{GetMaybeNet, MakePrimitive, PrimitiveIndex},
        primitive::MakePrimitiveShape,
        rules::AccessRules,
    },
    geometry::{compound::ManageCompounds, shape::AccessShape},
    graph::{GenericIndex, GetPetgraphIndex},
    layout::{
        poly::{MakePolyShape, PolyWeight},
        Layout,
    },
    triangulation::{GetTrianvertexNodeIndex, Triangulation},
};

#[enum_dispatch(GetPetgraphIndex)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RatvertexIndex {
    FixedDot(FixedDotIndex),
    Poly(GenericIndex<PolyWeight>),
}

impl From<RatvertexIndex> for crate::layout::NodeIndex {
    fn from(vertex: RatvertexIndex) -> crate::layout::NodeIndex {
        match vertex {
            RatvertexIndex::FixedDot(dot) => crate::layout::NodeIndex::Primitive(dot.into()),
            RatvertexIndex::Poly(poly) => crate::layout::NodeIndex::Compound(poly.into()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RatvertexWeight {
    vertex: RatvertexIndex,
    pub pos: Point,
}

impl GetTrianvertexNodeIndex<RatvertexIndex> for RatvertexWeight {
    fn node_index(&self) -> RatvertexIndex {
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
    pub band_termseg: Option<BandTermsegIndex>,
}

pub struct Ratsnest {
    graph: UnGraph<RatvertexWeight, RatlineWeight, usize>,
}

impl Ratsnest {
    pub fn new(layout: &Layout<impl AccessRules>) -> Result<Self, InsertionError> {
        let mut unionfind = UnionFind::new(layout.drawing().geometry().graph().node_bound());

        for edge in layout.drawing().geometry().graph().edge_references() {
            unionfind.union(edge.source(), edge.target());
        }

        let mut this = Self {
            graph: UnGraph::default(),
        };

        let mut triangulations = HashMap::new();
        let node_bound = layout.drawing().geometry().graph().node_bound();

        for layer in 0..layout.drawing().layer_count() {
            let mut handle_rvw = |maybe_net: Option<usize>, vertex: RatvertexIndex, pos: Point| {
                if let Some(net) = maybe_net {
                    triangulations
                        .entry((layer, net))
                        .or_insert_with(|| Triangulation::new(node_bound))
                        .add_vertex(RatvertexWeight { vertex, pos })?;
                }
                Ok(())
            };

            for node in layout.drawing().layer_primitive_nodes(layer) {
                if let PrimitiveIndex::FixedDot(dot) = node {
                    if layout.polys(dot).next().is_none() {
                        handle_rvw(
                            layout.drawing().primitive(dot).maybe_net(),
                            RatvertexIndex::FixedDot(dot),
                            node.primitive(layout.drawing()).shape().center(),
                        )?;
                    }
                }
            }

            for poly in layout.layer_poly_nodes(layer) {
                handle_rvw(
                    layout.drawing().compound_weight(poly.into()).maybe_net(),
                    RatvertexIndex::Poly(poly),
                    layout.poly(poly).shape().center(),
                )?;
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
                let source_index = g.node_weight(source).unwrap().node_index().petgraph_index();
                let target_index = g.node_weight(target).unwrap().node_index().petgraph_index();
                !unionfind.equiv(source_index, target_index)
            } else {
                true
            }
        });

        Ok(this)
    }

    pub fn assign_band_termseg_to_ratline(
        &mut self,
        ratline: EdgeIndex<usize>,
        termseg: BandTermsegIndex,
    ) {
        self.graph.edge_weight_mut(ratline).unwrap().band_termseg = Some(termseg);
    }

    pub fn graph(&self) -> &UnGraph<RatvertexWeight, RatlineWeight, usize> {
        &self.graph
    }
}
