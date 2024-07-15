use std::collections::HashMap;

use enum_dispatch::enum_dispatch;
use geo::Point;
use petgraph::{
    data::DataMap,
    graph::UnGraph,
    stable_graph::NodeIndex,
    visit::{
        Data, EdgeRef, GraphBase, IntoEdgeReferences, IntoEdges, IntoNeighbors,
        IntoNodeIdentifiers, NodeIndexable,
    },
};
use spade::{HasPosition, InsertionError, Point2};
use thiserror::Error;

use crate::{
    drawing::{
        bend::{FixedBendIndex, LooseBendIndex},
        dot::FixedDotIndex,
        graph::{GetLayer, GetMaybeNet, MakePrimitive, PrimitiveIndex},
        primitive::{MakePrimitiveShape, Primitive},
        rules::AccessRules,
        Drawing,
    },
    geometry::shape::AccessShape,
    graph::GetPetgraphIndex,
    layout::Layout,
    router::astar::MakeEdgeRef,
    triangulation::{GetTrianvertexNodeIndex, Triangulation},
};

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub struct NavvertexIndex(NodeIndex<usize>);

impl GetPetgraphIndex for NavvertexIndex {
    fn petgraph_index(&self) -> NodeIndex<usize> {
        self.0
    }
}

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
    pub maybe_cw: Option<bool>,
}

#[derive(Error, Debug, Clone)]
pub enum NavmeshError {
    #[error("failed to insert vertex in navmesh")]
    Insertion(#[from] InsertionError),
}

#[derive(Debug, Clone)]
pub struct Navmesh {
    graph: UnGraph<NavvertexWeight, (), usize>,
    origin: FixedDotIndex,
    origin_navvertex: NavvertexIndex,
    destination: FixedDotIndex,
    destination_navvertex: NavvertexIndex,
}

impl Navmesh {
    pub fn new(
        layout: &Layout<impl AccessRules>,
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
        let mut source_navvertex = None;
        let mut target_navvertex = None;

        // `HashMap` is obviously suboptimal here.
        let mut map = HashMap::new();

        for trianvertex in triangulation.node_identifiers() {
            let binavvertex = if trianvertex == source.into() {
                let navvertex = graph.add_node(NavvertexWeight {
                    node: trianvertex.into(),
                    maybe_cw: None,
                });

                source_navvertex = Some(navvertex);
                (navvertex, navvertex)
            } else if trianvertex == target.into() {
                let navvertex = graph.add_node(NavvertexWeight {
                    node: trianvertex.into(),
                    maybe_cw: None,
                });

                target_navvertex = Some(navvertex);
                (navvertex, navvertex)
            } else {
                let navvertex1 = graph.add_node(NavvertexWeight {
                    node: trianvertex.into(),
                    maybe_cw: Some(false),
                });

                let navvertex2 = graph.add_node(NavvertexWeight {
                    node: trianvertex.into(),
                    maybe_cw: Some(true),
                });

                (navvertex1, navvertex2)
            };

            map.insert(trianvertex, binavvertex);
        }

        for edge in triangulation.edge_references() {
            let (from_navvertex1, from_navvertex2) = map[&edge.source()];
            let (to_navvertex1, to_navvertex2) = map[&edge.target()];

            graph.update_edge(from_navvertex1, to_navvertex1, ());
            graph.update_edge(from_navvertex1, to_navvertex2, ());
            graph.update_edge(from_navvertex2, to_navvertex1, ());
            graph.update_edge(from_navvertex2, to_navvertex2, ());
        }

        Ok(Self {
            graph,
            origin: source,
            origin_navvertex: NavvertexIndex(source_navvertex.unwrap()),
            destination: target,
            destination_navvertex: NavvertexIndex(target_navvertex.unwrap()),
        })
    }

    pub fn graph(&self) -> &UnGraph<NavvertexWeight, (), usize> {
        &self.graph
    }

    pub fn origin(&self) -> FixedDotIndex {
        self.origin
    }

    pub fn origin_navvertex(&self) -> NavvertexIndex {
        self.origin_navvertex
    }

    pub fn destination(&self) -> FixedDotIndex {
        self.destination
    }

    pub fn destination_navvertex(&self) -> NavvertexIndex {
        self.destination_navvertex
    }
}

impl GraphBase for Navmesh {
    type NodeId = NavvertexIndex;
    type EdgeId = (NavvertexIndex, NavvertexIndex);
}

impl Data for Navmesh {
    type NodeWeight = NavvertexWeight;
    type EdgeWeight = ();
}

impl DataMap for Navmesh {
    fn node_weight(&self, vertex: Self::NodeId) -> Option<&Self::NodeWeight> {
        self.graph.node_weight(vertex.petgraph_index())
    }

    fn edge_weight(&self, _edge: Self::EdgeId) -> Option<&Self::EdgeWeight> {
        None
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NavmeshEdgeReference {
    from: NavvertexIndex,
    to: NavvertexIndex,
}

impl EdgeRef for NavmeshEdgeReference {
    type NodeId = NavvertexIndex;
    type EdgeId = (NavvertexIndex, NavvertexIndex);
    type Weight = ();

    fn source(&self) -> Self::NodeId {
        self.from
    }

    fn target(&self) -> Self::NodeId {
        self.to
    }

    fn weight(&self) -> &Self::Weight {
        &()
    }

    fn id(&self) -> Self::EdgeId {
        (self.from, self.to)
    }
}

impl<'a> IntoNeighbors for &'a Navmesh {
    type Neighbors = Box<dyn Iterator<Item = NavvertexIndex> + 'a>;

    fn neighbors(self, vertex: Self::NodeId) -> Self::Neighbors {
        Box::new(
            self.graph
                .neighbors(vertex.petgraph_index())
                .map(|ni| NavvertexIndex(ni)),
        )
    }
}

impl<'a> IntoEdgeReferences for &'a Navmesh {
    type EdgeRef = NavmeshEdgeReference;
    type EdgeReferences = Box<dyn Iterator<Item = NavmeshEdgeReference> + 'a>;

    fn edge_references(self) -> Self::EdgeReferences {
        Box::new(
            self.graph
                .edge_references()
                .map(|edge| NavmeshEdgeReference {
                    from: NavvertexIndex(edge.source()),
                    to: NavvertexIndex(edge.target()),
                }),
        )
    }
}

impl<'a> IntoEdges for &'a Navmesh {
    type Edges = Box<dyn Iterator<Item = NavmeshEdgeReference> + 'a>;

    fn edges(self, vertex: Self::NodeId) -> Self::Edges {
        Box::new(
            self.graph
                .edges(vertex.petgraph_index())
                .map(|edge| NavmeshEdgeReference {
                    from: NavvertexIndex(edge.source()),
                    to: NavvertexIndex(edge.target()),
                }),
        )
    }
}

impl<'a> MakeEdgeRef for &'a Navmesh {
    fn edge_ref(
        &self,
        edge_id: <&'a Navmesh as GraphBase>::EdgeId,
    ) -> <&'a Navmesh as IntoEdgeReferences>::EdgeRef {
        NavmeshEdgeReference {
            from: edge_id.0,
            to: edge_id.1,
        }
    }
}
