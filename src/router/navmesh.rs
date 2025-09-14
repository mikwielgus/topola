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
    unionfind::UnionFind,
    visit::{
        Data, EdgeRef, GraphBase, IntoEdgeReferences, IntoEdges, IntoNeighbors,
        IntoNodeIdentifiers, Walker,
    },
};
use spade::InsertionError;
use thiserror::Error;

use crate::{
    drawing::{
        bend::{FixedBendIndex, LooseBendIndex},
        dot::FixedDotIndex,
        gear::{GearIndex, GetOuterGears, WalkOutwards},
        graph::PrimitiveIndex,
        rules::AccessRules,
    },
    graph::{GenericIndex, GetIndex, MakeRef},
    layout::{CompoundEntryLabel, Layout},
    math::RotationSense,
    router::thetastar::MakeEdgeRef,
};

use super::{
    prenavmesh::{Prenavmesh, PrenavmeshNodeIndex},
    RouterOptions,
};

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct NavnodeIndex(pub NodeIndex<usize>);

impl core::fmt::Debug for NavnodeIndex {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "NavnodeIndex({})", self.0.index())
    }
}

impl GetIndex for NavnodeIndex {
    fn index(&self) -> usize {
        self.0.index()
    }
}

/// A binavnode is a pair of navnodes, one clockwise and the other
/// counterclockwise. Unlike their constituents, binavnodes are themselves
/// not considered navnodes.
#[enum_dispatch(GetIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinavnodeNodeIndex {
    FixedDot(FixedDotIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

impl From<PrenavmeshNodeIndex> for BinavnodeNodeIndex {
    fn from(node: PrenavmeshNodeIndex) -> Self {
        match node {
            PrenavmeshNodeIndex::FixedDot(dot) => BinavnodeNodeIndex::FixedDot(dot),
            PrenavmeshNodeIndex::FixedBend(bend) => BinavnodeNodeIndex::FixedBend(bend),
        }
    }
}

impl From<BinavnodeNodeIndex> for PrimitiveIndex {
    fn from(node: BinavnodeNodeIndex) -> Self {
        match node {
            BinavnodeNodeIndex::FixedDot(dot) => PrimitiveIndex::FixedDot(dot),
            BinavnodeNodeIndex::FixedBend(bend) => PrimitiveIndex::FixedBend(bend),
            BinavnodeNodeIndex::LooseBend(bend) => PrimitiveIndex::LooseBend(bend),
        }
    }
}

impl From<BinavnodeNodeIndex> for GearIndex {
    fn from(node: BinavnodeNodeIndex) -> Self {
        match node {
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
    pub binavnode: BinavnodeNodeIndex,

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

        let mut prenavnode_to_navnodes = BTreeMap::new();
        let mut overlapping_prenavnodes_unions =
        // FIXME: This bound is incorrect. You can't actually unionize dots with bends.
            UnionFind::new(layout.drawing().geometry().dot_index_bound());

        for prenavnode in prenavmesh.triangulation().node_identifiers() {
            if prenavnode == origin.into() {
                let navnode = graph.add_node(NavnodeWeight {
                    binavnode: prenavnode.into(),
                    maybe_sense: None,
                });

                origin_navnode = Some(navnode);
                prenavnode_to_navnodes.insert(prenavnode, vec![(navnode, navnode)]);
            } else if prenavnode == destination.into() {
                let navnode = graph.add_node(NavnodeWeight {
                    binavnode: prenavnode.into(),
                    maybe_sense: None,
                });

                destination_navnode = Some(navnode);
                prenavnode_to_navnodes.insert(prenavnode, vec![(navnode, navnode)]);
            } else {
                prenavnode_to_navnodes.insert(prenavnode, vec![]);
                Self::unionize_with_overlapees(
                    layout,
                    &mut overlapping_prenavnodes_unions,
                    prenavnode,
                );

                let gear = Into::<GearIndex>::into(Into::<BinavnodeNodeIndex>::into(prenavnode));

                if options.squeeze_through_under_bends {
                    Self::add_prenavnode_as_binavnode(
                        &mut graph,
                        &mut prenavnode_to_navnodes,
                        prenavnode,
                        prenavnode.into(),
                    );

                    if options.wrap_around_bands {
                        let mut outwards = gear.ref_(layout.drawing()).outwards();
                        while let Some(outward) = outwards.walk_next(layout.drawing()) {
                            Self::add_prenavnode_as_binavnode(
                                &mut graph,
                                &mut prenavnode_to_navnodes,
                                prenavnode,
                                outward.into(),
                            );
                        }
                    }
                } else if !gear.ref_(layout.drawing()).outer_gears().is_empty() {
                    let mut outwards = gear.ref_(layout.drawing()).outwards();
                    while let Some(outward) = outwards.walk_next(layout.drawing()) {
                        if layout
                            .drawing()
                            .primitive(outward)
                            .outers()
                            .collect::<Vec<_>>()
                            .is_empty()
                        {
                            Self::add_prenavnode_as_binavnode(
                                &mut graph,
                                &mut prenavnode_to_navnodes,
                                prenavnode,
                                outward.into(),
                            );
                        }
                    }
                } else {
                    Self::add_prenavnode_as_binavnode(
                        &mut graph,
                        &mut prenavnode_to_navnodes,
                        prenavnode,
                        prenavnode.into(),
                    );
                }
            }
        }

        for prenavedge in prenavmesh.triangulation().edge_references() {
            Self::add_prenavedge_to_repr_as_quadrinavedges(
                &mut graph,
                &prenavnode_to_navnodes,
                &overlapping_prenavnodes_unions,
                prenavedge.source(),
                prenavedge.target(),
            );
        }

        // The existence of a constraint edge does not (!) guarantee that this
        // edge exactly will be present in the triangulation. It appears that
        // Spade splits a constraint edge in two if an endpoint of another
        // constraint lies on it.
        //
        // So now we go over all the constraints and make sure that
        // quadrinavedges exist for every one of them.
        for constraint in prenavmesh.constraints() {
            Self::add_prenavedge_to_repr_as_quadrinavedges(
                &mut graph,
                &prenavnode_to_navnodes,
                &overlapping_prenavnodes_unions,
                constraint.0.node,
                constraint.1.node,
            );
        }

        // Copy prenavedges of prenavnodes union representatives to all elements
        // for each union.
        for prenavnode in prenavmesh.triangulation().node_identifiers() {
            let repr = PrenavmeshNodeIndex::FixedDot(GenericIndex::new(
                overlapping_prenavnodes_unions.find(prenavnode.index()),
            ));

            if repr == prenavnode {
                continue;
            }

            for (repr_navnode1, repr_navnode2) in prenavnode_to_navnodes[&repr].iter() {
                let repr_navedge1_targets: Vec<_> = graph
                    .edges(*repr_navnode1)
                    .map(|edge| edge.target())
                    .collect();

                for target in repr_navedge1_targets {
                    for (from_navnode1, from_navnode2) in prenavnode_to_navnodes[&prenavnode].iter()
                    {
                        graph.update_edge(*from_navnode1, target, ());
                        graph.update_edge(*from_navnode2, target, ());
                    }
                }

                let repr_navedge2_targets: Vec<_> = graph
                    .edges(*repr_navnode2)
                    .map(|edge| edge.target())
                    .collect();

                for target in repr_navedge2_targets {
                    for (from_navnode1, from_navnode2) in prenavnode_to_navnodes[&prenavnode].iter()
                    {
                        graph.update_edge(*from_navnode1, target, ());
                        graph.update_edge(*from_navnode2, target, ());
                    }
                }
            }
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

    fn unionize_with_overlapees(
        layout: &Layout<impl AccessRules>,
        overlapping_prenavnodes_unions: &mut UnionFind<usize>,
        prenavnode: PrenavmeshNodeIndex,
    ) {
        // Ignore overlaps of a fillet.
        if layout
            .drawing()
            .compounds(GenericIndex::<()>::new(prenavnode.index()))
            .find(|(label, _)| *label == CompoundEntryLabel::Fillet)
            .is_some()
        {
            return;
        }

        for overlapee in layout.drawing().overlapees(prenavnode.into()) {
            // Ignore overlaps with fillets.
            if layout
                .drawing()
                .compounds(GenericIndex::<()>::new(overlapee.1.index()))
                .find(|(label, _)| *label == CompoundEntryLabel::Fillet)
                .is_some()
            {
                continue;
            }

            let PrimitiveIndex::FixedDot(overlapee) = overlapee.1 else {
                continue;
            };

            overlapping_prenavnodes_unions.union(prenavnode.index(), overlapee.index());
        }
    }

    fn add_prenavnode_as_binavnode(
        graph: &mut UnGraph<NavnodeWeight, (), usize>,
        prenavnode_to_navnodes: &mut BTreeMap<
            PrenavmeshNodeIndex,
            Vec<(NodeIndex<usize>, NodeIndex<usize>)>,
        >,
        trianvertex: PrenavmeshNodeIndex,
        node: BinavnodeNodeIndex,
    ) {
        let navnode1 = graph.add_node(NavnodeWeight {
            binavnode: node,
            maybe_sense: Some(RotationSense::Counterclockwise),
        });

        let navnode2 = graph.add_node(NavnodeWeight {
            binavnode: node,
            maybe_sense: Some(RotationSense::Clockwise),
        });

        prenavnode_to_navnodes
            .get_mut(&trianvertex)
            .unwrap()
            .push((navnode1, navnode2));
    }

    fn add_prenavedge_to_repr_as_quadrinavedges(
        graph: &mut UnGraph<NavnodeWeight, (), usize>,
        prenavnode_to_navnodes: &BTreeMap<
            PrenavmeshNodeIndex,
            Vec<(NodeIndex<usize>, NodeIndex<usize>)>,
        >,
        overlapping_prenavnodes_unions: &UnionFind<usize>,
        from_prenavnode: PrenavmeshNodeIndex,
        to_prenavnode: PrenavmeshNodeIndex,
    ) {
        // We assume prenavmesh nodes are fixed dots. This is an ugly shortcut,
        // since fixed bends also can be prenavnodes, but it works for now.
        let from_prenavnode_repr = PrenavmeshNodeIndex::FixedDot(GenericIndex::new(
            overlapping_prenavnodes_unions.find(from_prenavnode.index().into()),
        ));
        let to_prenavnode_repr = PrenavmeshNodeIndex::FixedDot(GenericIndex::new(
            overlapping_prenavnodes_unions.find(to_prenavnode.index().into()),
        ));

        Self::add_prenavedge_as_quadrinavedges(
            graph,
            prenavnode_to_navnodes,
            from_prenavnode_repr,
            to_prenavnode_repr,
        )
    }

    fn add_prenavedge_as_quadrinavedges(
        graph: &mut UnGraph<NavnodeWeight, (), usize>,
        prenavnode_to_navnodes: &BTreeMap<
            PrenavmeshNodeIndex,
            Vec<(NodeIndex<usize>, NodeIndex<usize>)>,
        >,
        from_prenavnode: PrenavmeshNodeIndex,
        to_prenavnode: PrenavmeshNodeIndex,
    ) {
        for (from_navnode1, from_navnode2) in prenavnode_to_navnodes[&from_prenavnode].iter() {
            for (to_navnode1, to_navnode2) in prenavnode_to_navnodes[&to_prenavnode].iter() {
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
        self.graph.node_weight(vertex.index().into())
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
                .neighbors(vertex.index().into())
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
                .edges(vertex.index().into())
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
