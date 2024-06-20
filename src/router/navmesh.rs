use std::collections::HashMap;

use enum_dispatch::enum_dispatch;
use geo::Point;
use petgraph::{
    graph::UnGraph,
    stable_graph::NodeIndex,
    visit::{EdgeRef, IntoEdgeReferences, IntoEdges, IntoNodeIdentifiers, NodeIndexable},
};
use spade::{HasPosition, InsertionError, Point2};
use thiserror::Error;

use crate::{
    drawing::{
        bend::{FixedBendIndex, LooseBendIndex},
        dot::FixedDotIndex,
        graph::{GetLayer, GetMaybeNet, MakePrimitive, PrimitiveIndex},
        primitive::{MakePrimitiveShape, Primitive},
        rules::RulesTrait,
        Drawing,
    },
    geometry::shape::ShapeTrait,
    graph::GetPetgraphIndex,
    layout::Layout,
    triangulation::{GetTrianvertexNodeIndex, Triangulation},
};

#[enum_dispatch(GetPetgraphIndex, MakePrimitive)]
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum BinavvertexNodeIndex {
    FixedDot(FixedDotIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

impl From<BinavvertexNodeIndex> for PrimitiveIndex {
    fn from(vertex: BinavvertexNodeIndex) -> Self {
        match vertex {
            BinavvertexNodeIndex::FixedDot(dot) => PrimitiveIndex::FixedDot(dot),
            BinavvertexNodeIndex::FixedBend(bend) => PrimitiveIndex::FixedBend(bend),
            BinavvertexNodeIndex::LooseBend(bend) => PrimitiveIndex::LooseBend(bend),
        }
    }
}

#[enum_dispatch(GetPetgraphIndex, MakePrimitive)]
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
enum TrianvertexNodeIndex {
    FixedDot(FixedDotIndex),
    FixedBend(FixedBendIndex),
}

impl From<TrianvertexNodeIndex> for BinavvertexNodeIndex {
    fn from(vertex: TrianvertexNodeIndex) -> Self {
        match vertex {
            TrianvertexNodeIndex::FixedDot(dot) => BinavvertexNodeIndex::FixedDot(dot),
            TrianvertexNodeIndex::FixedBend(bend) => BinavvertexNodeIndex::FixedBend(bend),
        }
    }
}

#[derive(Debug, Clone)]
struct TrianvertexWeight {
    pub node: TrianvertexNodeIndex,
    pub pos: Point,
}

impl GetTrianvertexNodeIndex<TrianvertexNodeIndex> for TrianvertexWeight {
    fn node_index(&self) -> TrianvertexNodeIndex {
        self.node
    }
}

impl HasPosition for TrianvertexWeight {
    type Scalar = f64;
    fn position(&self) -> Point2<Self::Scalar> {
        Point2::new(self.pos.x(), self.pos.y())
    }
}

#[derive(Debug, Clone)]
pub struct NavvertexWeight {
    pub node: BinavvertexNodeIndex,
}

#[derive(Error, Debug, Clone)]
pub enum NavmeshError {
    #[error("failed to insert vertex in navmesh")]
    Insertion(#[from] InsertionError),
}

#[derive(Debug, Clone)]
pub struct Navmesh {
    graph: UnGraph<NavvertexWeight, (), usize>,
    source: FixedDotIndex,
    source_vertex: NodeIndex<usize>,
    target: FixedDotIndex,
    target_vertex: NodeIndex<usize>,
}

impl Navmesh {
    pub fn new(
        layout: &Layout<impl RulesTrait>,
        source: FixedDotIndex,
        target: FixedDotIndex,
    ) -> Result<Self, NavmeshError> {
        let mut triangulation: Triangulation<TrianvertexNodeIndex, TrianvertexWeight, ()> =
            Triangulation::new(layout.drawing().geometry().graph().node_bound());

        let layer = layout.drawing().primitive(source).layer();
        let maybe_net = layout.drawing().primitive(source).maybe_net();

        for node in layout.drawing().layer_primitive_nodes(layer) {
            let primitive = node.primitive(layout.drawing());

            if let Some(primitive_net) = primitive.maybe_net() {
                if node == source.into()
                    || node == target.into()
                    || Some(primitive_net) != maybe_net
                {
                    match node {
                        PrimitiveIndex::FixedDot(dot) => {
                            triangulation.add_vertex(TrianvertexWeight {
                                node: dot.into(),
                                pos: primitive.shape().center(),
                            })?;
                        }
                        PrimitiveIndex::FixedBend(bend) => {
                            triangulation.add_vertex(TrianvertexWeight {
                                node: bend.into(),
                                pos: primitive.shape().center(),
                            })?;
                        }
                        _ => (),
                    }
                }
            }
        }

        let mut graph: UnGraph<NavvertexWeight, (), usize> = UnGraph::default();
        let mut source_vertex = None;
        let mut target_vertex = None;

        // `HashMap` is obviously suboptimal here.
        let mut map = HashMap::new();

        for trianvertex in triangulation.node_identifiers() {
            let navvertex = graph.add_node(NavvertexWeight {
                node: trianvertex.into(),
            });

            if trianvertex == source.into() {
                source_vertex = Some(navvertex);
            } else if trianvertex == target.into() {
                target_vertex = Some(navvertex);
            }

            map.insert(trianvertex, navvertex);
        }

        for edge in triangulation.edge_references() {
            graph.add_edge(map[&edge.source()], map[&edge.target()], ());
        }

        Ok(Self {
            graph,
            source,
            source_vertex: source_vertex.unwrap(),
            target,
            target_vertex: target_vertex.unwrap(),
        })
    }

    pub fn graph(&self) -> &UnGraph<NavvertexWeight, (), usize> {
        &self.graph
    }

    pub fn source(&self) -> FixedDotIndex {
        self.source
    }

    pub fn source_vertex(&self) -> NodeIndex<usize> {
        self.source_vertex
    }

    pub fn target(&self) -> FixedDotIndex {
        self.target
    }

    pub fn target_vertex(&self) -> NodeIndex<usize> {
        self.target_vertex
    }
}
