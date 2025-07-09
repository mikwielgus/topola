// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use derive_getters::Getters;
use enum_dispatch::enum_dispatch;
use petgraph::{
    data::DataMap,
    graph::UnGraph,
    stable_graph::NodeIndex,
    visit::{
        Data, EdgeRef, GraphBase, IntoEdgeReferences, IntoEdges, IntoNeighbors,
        IntoNodeIdentifiers, NodeIndexable,
    },
};
use spade::InsertionError;
use thiserror::Error;

use crate::{
    drawing::{
        bend::{FixedBendIndex, LooseBendIndex},
        dot::FixedDotIndex,
        gear::{GearIndex, GetNextGear},
        graph::{GetMaybeNet, MakePrimitive, PrimitiveIndex},
        primitive::Primitive,
        rules::AccessRules,
        Drawing,
    },
    geometry::GetLayer,
    graph::{GetPetgraphIndex, MakeRef},
    layout::Layout,
    math::RotationSense,
    router::thetastar::MakeEdgeRef,
    triangulation::Triangulation,
};

use super::{
    prenavmesh::{Prenavmesh, PrenavmeshConstraint, PrenavmeshNodeIndex, PrenavmeshWeight},
    RouterOptions,
};

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct NavnodeIndex(pub NodeIndex<usize>);

impl core::fmt::Debug for NavnodeIndex {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "NavnodeIndex({})", self.0.index())
    }
}

impl GetPetgraphIndex for NavnodeIndex {
    fn petgraph_index(&self) -> NodeIndex<usize> {
        self.0
    }
}

/// A binavnode is a pair of navnodes, one clockwise and the other
/// counterclockwise. Unlike their constituents, binavnodes are themselves
/// not considered navnodes.
#[enum_dispatch(GetPetgraphIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinavnodeNodeIndex {
    FixedDot(FixedDotIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

impl From<PrenavmeshNodeIndex> for BinavnodeNodeIndex {
    fn from(trianvertex: PrenavmeshNodeIndex) -> Self {
        match trianvertex {
            PrenavmeshNodeIndex::FixedDot(dot) => BinavnodeNodeIndex::FixedDot(dot),
            PrenavmeshNodeIndex::FixedBend(bend) => BinavnodeNodeIndex::FixedBend(bend),
        }
    }
}

impl From<BinavnodeNodeIndex> for PrimitiveIndex {
    fn from(vertex: BinavnodeNodeIndex) -> Self {
        match vertex {
            BinavnodeNodeIndex::FixedDot(dot) => PrimitiveIndex::FixedDot(dot),
            BinavnodeNodeIndex::FixedBend(bend) => PrimitiveIndex::FixedBend(bend),
            BinavnodeNodeIndex::LooseBend(bend) => PrimitiveIndex::LooseBend(bend),
        }
    }
}

impl From<BinavnodeNodeIndex> for GearIndex {
    fn from(vertex: BinavnodeNodeIndex) -> Self {
        match vertex {
            BinavnodeNodeIndex::FixedDot(dot) => GearIndex::FixedDot(dot),
            BinavnodeNodeIndex::FixedBend(bend) => GearIndex::FixedBend(bend),
            BinavnodeNodeIndex::LooseBend(bend) => GearIndex::LooseBend(bend),
        }
    }
}

/// The terms "navnode" and "navmesh vertex", "navmesh node", "navigation
/// vertex", "navigation node" are all equivalent.
///
/// See the following blog post for more information and a visualization of the navmesh
/// during autorouting: <https://topola.dev/blog/2024/07/20/junejuly-2024-development-update/#advanced-debug-visualization>
#[derive(Debug, Clone)]
pub struct NavnodeWeight {
    pub node: BinavnodeNodeIndex,

    /// There are two navnodes for each navigable node:
    /// one is clockwise (`Some(true)`), the other counterclockwise (`Some(false)`).
    /// The origin and destination nodes however have
    /// only one corresponding navmesh vertex each (`None`).
    pub maybe_sense: Option<RotationSense>,
}

#[derive(Error, Debug, Clone)]
pub enum NavmeshError {
    #[error("failed to insert vertex in navmesh")]
    Insertion(#[from] InsertionError),
}

/// The navmesh holds the entire Topola's search space represented as a graph.
/// Topola's routing works by navigating over this graph with a pathfinding
/// algorithm such as A* while drawing a track segment (always a cane except
/// when going directly to destination) on the layout for each leap and
/// along-edge crossing.
///
/// The name "navmesh" is a blend of "navigation mesh".
#[derive(Clone, Getters)]
pub struct Navmesh {
    graph: UnGraph<NavnodeWeight, (), usize>,
    #[getter(skip)]
    origin: FixedDotIndex,
    #[getter(skip)]
    origin_navnode: NavnodeIndex,
    #[getter(skip)]
    destination: FixedDotIndex,
    #[getter(skip)]
    destination_navnode: NavnodeIndex,

    /// Original constrainted triangulation stored for debugging purposes.
    prenavmesh: Prenavmesh,
}

impl Navmesh {
    /// Creates a new navmesh.
    pub fn new(
        layout: &Layout<impl AccessRules>,
        origin: FixedDotIndex,
        destination: FixedDotIndex,
        options: RouterOptions,
    ) -> Result<Self, NavmeshError> {
        let prenavmesh = Prenavmesh::new(layout, origin, destination, options)?;
        Self::new_from_prenavmesh(layout, prenavmesh, origin, destination, options)
    }

    fn new_from_prenavmesh(
        layout: &Layout<impl AccessRules>,
        prenavmesh: Prenavmesh,
        origin: FixedDotIndex,
        destination: FixedDotIndex,
        options: RouterOptions,
    ) -> Result<Self, NavmeshError> {
        let mut graph: UnGraph<NavnodeWeight, (), usize> = UnGraph::default();
        let mut origin_navnode = None;
        let mut destination_navnode = None;

        let mut map = BTreeMap::new();

        for trianvertex in prenavmesh.triangulation().node_identifiers() {
            if trianvertex == origin.into() {
                let navnode = graph.add_node(NavnodeWeight {
                    node: trianvertex.into(),
                    maybe_sense: None,
                });

                origin_navnode = Some(navnode);
                map.insert(trianvertex, vec![(navnode, navnode)]);
            } else if trianvertex == destination.into() {
                let navnode = graph.add_node(NavnodeWeight {
                    node: trianvertex.into(),
                    maybe_sense: None,
                });

                destination_navnode = Some(navnode);
                map.insert(trianvertex, vec![(navnode, navnode)]);
            } else {
                map.insert(trianvertex, vec![]);

                let mut gear =
                    Into::<GearIndex>::into(Into::<BinavnodeNodeIndex>::into(trianvertex));

                if options.squeeze_through_under_bends {
                    Self::add_trianvertex_to_graph_and_map_as_binavnode(
                        &mut graph,
                        &mut map,
                        trianvertex,
                        trianvertex.into(),
                    );

                    if options.wrap_around_bands {
                        while let Some(bend) = gear.ref_(layout.drawing()).next_gear() {
                            Self::add_trianvertex_to_graph_and_map_as_binavnode(
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

                    Self::add_trianvertex_to_graph_and_map_as_binavnode(
                        &mut graph,
                        &mut map,
                        trianvertex,
                        bend.into(),
                    );
                } else {
                    Self::add_trianvertex_to_graph_and_map_as_binavnode(
                        &mut graph,
                        &mut map,
                        trianvertex,
                        trianvertex.into(),
                    );
                }
            }
        }

        for edge in prenavmesh.triangulation().edge_references() {
            Self::add_trianedge_to_graph_as_quadrinavedge(
                &mut graph,
                &map,
                edge.source(),
                edge.target(),
            );
        }

        // The existence of a constraint edge does not (!) guarantee that this
        // edge exactly will be present in the triangulation. It appears that
        // Spade splits a constraint edge into two if an endpoint of another
        // constraint lies on it.
        //
        // So now we go over all the constraints and make sure that
        // quadrinavedges exist for every one of them.
        for constraint in prenavmesh.constraints() {
            Self::add_trianedge_to_graph_as_quadrinavedge(
                &mut graph,
                &map,
                constraint.0.node,
                constraint.1.node,
            );
        }

        Ok(Self {
            graph,
            origin,
            origin_navnode: NavnodeIndex(origin_navnode.unwrap()),
            destination,
            destination_navnode: NavnodeIndex(destination_navnode.unwrap()),
            prenavmesh,
        })
    }

    fn add_trianvertex_to_graph_and_map_as_binavnode(
        graph: &mut UnGraph<NavnodeWeight, (), usize>,
        map: &mut BTreeMap<PrenavmeshNodeIndex, Vec<(NodeIndex<usize>, NodeIndex<usize>)>>,
        trianvertex: PrenavmeshNodeIndex,
        node: BinavnodeNodeIndex,
    ) {
        let navnode1 = graph.add_node(NavnodeWeight {
            node,
            maybe_sense: Some(RotationSense::Counterclockwise),
        });

        let navnode2 = graph.add_node(NavnodeWeight {
            node,
            maybe_sense: Some(RotationSense::Clockwise),
        });

        map.get_mut(&trianvertex)
            .unwrap()
            .push((navnode1, navnode2));
    }

    fn add_trianedge_to_graph_as_quadrinavedge(
        graph: &mut UnGraph<NavnodeWeight, (), usize>,
        map: &BTreeMap<PrenavmeshNodeIndex, Vec<(NodeIndex<usize>, NodeIndex<usize>)>>,
        from_trianvertex: PrenavmeshNodeIndex,
        to_trianvertex: PrenavmeshNodeIndex,
    ) {
        for (from_navnode1, from_navnode2) in map[&from_trianvertex].iter() {
            for (to_navnode1, to_navnode2) in map[&to_trianvertex].iter() {
                graph.update_edge(*from_navnode1, *to_navnode1, ());
                graph.update_edge(*from_navnode1, *to_navnode2, ());
                graph.update_edge(*from_navnode2, *to_navnode1, ());
                graph.update_edge(*from_navnode2, *to_navnode2, ());
            }
        }
    }

    /// Returns the origin node.
    pub fn origin(&self) -> FixedDotIndex {
        self.origin
    }

    /// Returns the navnode of the origin node.
    pub fn origin_navnode(&self) -> NavnodeIndex {
        self.origin_navnode
    }

    /// Returns the destination node.
    pub fn destination(&self) -> FixedDotIndex {
        self.destination
    }

    /// Returns the navnode of the destination node.
    pub fn destination_navnode(&self) -> NavnodeIndex {
        self.destination_navnode
    }
}

impl GraphBase for Navmesh {
    type NodeId = NavnodeIndex;
    type EdgeId = (NavnodeIndex, NavnodeIndex);
}

impl Data for Navmesh {
    type NodeWeight = NavnodeWeight;
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
    from: NavnodeIndex,
    to: NavnodeIndex,
}

impl EdgeRef for NavmeshEdgeReference {
    type NodeId = NavnodeIndex;
    type EdgeId = (NavnodeIndex, NavnodeIndex);
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
    type Neighbors = Box<dyn Iterator<Item = NavnodeIndex> + 'a>;

    fn neighbors(self, vertex: Self::NodeId) -> Self::Neighbors {
        Box::new(
            self.graph
                .neighbors(vertex.petgraph_index())
                .map(NavnodeIndex),
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
                    from: NavnodeIndex(edge.source()),
                    to: NavnodeIndex(edge.target()),
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
                    from: NavnodeIndex(edge.source()),
                    to: NavnodeIndex(edge.target()),
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
