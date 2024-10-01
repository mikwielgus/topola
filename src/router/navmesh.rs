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
        gear::{GearIndex, GetNextGear},
        graph::{GetLayer, GetMaybeNet, MakePrimitive, PrimitiveIndex},
        primitive::{MakePrimitiveShape, Primitive},
        rules::AccessRules,
        Drawing,
    },
    geometry::shape::AccessShape,
    graph::{GetPetgraphIndex, MakeRef},
    layout::Layout,
    router::astar::MakeEdgeRef,
    triangulation::{GetTrianvertexNodeIndex, Triangulation},
};

use super::RouterOptions;

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

impl From<BinavvertexNodeIndex> for GearIndex {
    fn from(vertex: BinavvertexNodeIndex) -> Self {
        match vertex {
            BinavvertexNodeIndex::FixedDot(dot) => GearIndex::FixedDot(dot),
            BinavvertexNodeIndex::FixedBend(bend) => GearIndex::FixedBend(bend),
            BinavvertexNodeIndex::LooseBend(bend) => GearIndex::LooseBend(bend),
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
        origin: FixedDotIndex,
        destination: FixedDotIndex,
        options: RouterOptions,
    ) -> Result<Self, NavmeshError> {
        let mut triangulation: Triangulation<TrianvertexNodeIndex, TrianvertexWeight, ()> =
            Triangulation::new(layout.drawing().geometry().graph().node_bound());

        let layer = layout.drawing().primitive(origin).layer();
        let maybe_net = layout.drawing().primitive(origin).maybe_net();

        for node in layout.drawing().layer_primitive_nodes(layer) {
            let primitive = node.primitive(layout.drawing());

            if let Some(primitive_net) = primitive.maybe_net() {
                if node == origin.into()
                    || node == destination.into()
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

        Self::new_from_triangulation(layout, triangulation, origin, destination, options)
    }

    fn new_from_triangulation(
        layout: &Layout<impl AccessRules>,
        triangulation: Triangulation<TrianvertexNodeIndex, TrianvertexWeight, ()>,
        origin: FixedDotIndex,
        destination: FixedDotIndex,
        options: RouterOptions,
    ) -> Result<Self, NavmeshError> {
        let mut graph: UnGraph<NavvertexWeight, (), usize> = UnGraph::default();
        let mut origin_navvertex = None;
        let mut destination_navvertex = None;

        // `HashMap` is obviously suboptimal here.
        let mut map = HashMap::new();

        for trianvertex in triangulation.node_identifiers() {
            if trianvertex == origin.into() {
                let navvertex = graph.add_node(NavvertexWeight {
                    node: trianvertex.into(),
                    maybe_cw: None,
                });

                origin_navvertex = Some(navvertex);
                map.insert(trianvertex, vec![(navvertex, navvertex)]);
            } else if trianvertex == destination.into() {
                let navvertex = graph.add_node(NavvertexWeight {
                    node: trianvertex.into(),
                    maybe_cw: None,
                });

                destination_navvertex = Some(navvertex);
                map.insert(trianvertex, vec![(navvertex, navvertex)]);
            } else {
                map.insert(trianvertex, vec![]);

                let mut gear =
                    Into::<GearIndex>::into(Into::<BinavvertexNodeIndex>::into(trianvertex));

                if options.squeeze_through_under_bands {
                    Self::add_node_to_graph_and_map_as_binavvertex(
                        &mut graph,
                        &mut map,
                        trianvertex,
                        trianvertex.into(),
                    );

                    if options.wrap_around_bands {
                        while let Some(bend) = gear.ref_(layout.drawing()).next_gear() {
                            Self::add_node_to_graph_and_map_as_binavvertex(
                                &mut graph,
                                &mut map,
                                trianvertex,
                                bend.into(),
                            );
                            gear = bend.into();
                        }
                    }
                } else if let Some(first_bend) = gear.ref_(layout.drawing()).next_gear() {
                    let mut bend = first_bend;

                    while let Some(next_bend) = gear.ref_(layout.drawing()).next_gear() {
                        bend = next_bend;
                        gear = bend.into();
                    }

                    Self::add_node_to_graph_and_map_as_binavvertex(
                        &mut graph,
                        &mut map,
                        trianvertex,
                        bend.into(),
                    );
                } else {
                    Self::add_node_to_graph_and_map_as_binavvertex(
                        &mut graph,
                        &mut map,
                        trianvertex,
                        trianvertex.into(),
                    );
                }
            };
        }

        for edge in triangulation.edge_references() {
            for (from_navvertex1, from_navvertex2) in map[&edge.source()].iter() {
                for (to_navvertex1, to_navvertex2) in map[&edge.target()].iter() {
                    graph.update_edge(*from_navvertex1, *to_navvertex1, ());
                    graph.update_edge(*from_navvertex1, *to_navvertex2, ());
                    graph.update_edge(*from_navvertex2, *to_navvertex1, ());
                    graph.update_edge(*from_navvertex2, *to_navvertex2, ());
                }
            }
        }

        Ok(Self {
            graph,
            origin,
            origin_navvertex: NavvertexIndex(origin_navvertex.unwrap()),
            destination,
            destination_navvertex: NavvertexIndex(destination_navvertex.unwrap()),
        })
    }

    fn add_node_to_graph_and_map_as_binavvertex(
        graph: &mut UnGraph<NavvertexWeight, (), usize>,
        map: &mut HashMap<TrianvertexNodeIndex, Vec<(NodeIndex<usize>, NodeIndex<usize>)>>,
        trianvertex: TrianvertexNodeIndex,
        node: BinavvertexNodeIndex,
    ) {
        let navvertex1 = graph.add_node(NavvertexWeight {
            node,
            maybe_cw: Some(false),
        });

        let navvertex2 = graph.add_node(NavvertexWeight {
            node,
            maybe_cw: Some(true),
        });

        map.get_mut(&trianvertex)
            .unwrap()
            .push((navvertex1, navvertex2));
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
                .map(NavvertexIndex),
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
