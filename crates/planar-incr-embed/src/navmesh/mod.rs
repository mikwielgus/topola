// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT
//
//! A topological navmesh implementation
// idea: see issue topola/topola#132

use alloc::collections::BTreeMap;
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::mem::take;

use crate::{mayrev::MaybeReversed, planarr, Edge, NavmeshBase, NavmeshIndex, Node, RelaxedPath};

mod generate;
mod ordered_pair;

pub use generate::TrianVertex;
pub use ordered_pair::OrderedPair;

pub type EdgeIndex<T> = ordered_pair::OrderedPair<T>;
pub type EdgePaths<EP, CT> = Arc<[RelaxedPath<EP, CT>]>;

/// A topological navmesh (here in comfortably serializable form),
/// built upon the merging of the primary and dual graph,
/// and with an operation to create barriers.
///
/// This is basically a planar graph embedding.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        deserialize = "B: NavmeshBase,
            B::PrimalNodeIndex: serde::Deserialize<'de>,
            B::EtchedPath: serde::Deserialize<'de>,
            B::GapComment: serde::Deserialize<'de>,
            B::Scalar: serde::Deserialize<'de>",
        serialize = "B: NavmeshBase,
            B::PrimalNodeIndex: serde::Serialize,
            B::EtchedPath: serde::Serialize,
            B::GapComment: serde::Serialize,
            B::Scalar: serde::Serialize"
    ))
)]
pub struct NavmeshSer<B: NavmeshBase> {
    pub nodes: Arc<BTreeMap<NavmeshIndex<B::PrimalNodeIndex>, Node<B::PrimalNodeIndex, B::Scalar>>>,
    pub edges: BTreeMap<
        EdgeIndex<NavmeshIndex<B::PrimalNodeIndex>>,
        (
            Edge<B::PrimalNodeIndex>,
            EdgePaths<B::EtchedPath, B::GapComment>,
        ),
    >,
}

/// A topological navmesh,
/// built upon the merging of the primary and dual graph,
/// and with an operation to create barriers.
///
/// This is basically a planar graph embedding, and is used for enumeration of such embeddings.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        deserialize = "B: NavmeshBase,
            B::PrimalNodeIndex: serde::Deserialize<'de>,
            B::EtchedPath: serde::Deserialize<'de>,
            B::GapComment: serde::Deserialize<'de>,
            B::Scalar: serde::Deserialize<'de>",
        serialize = "B: NavmeshBase,
            B::PrimalNodeIndex: serde::Serialize,
            B::EtchedPath: serde::Serialize,
            B::GapComment: serde::Serialize,
            B::Scalar: serde::Serialize"
    ))
)]
pub struct Navmesh<B: NavmeshBase> {
    pub nodes: Arc<BTreeMap<NavmeshIndex<B::PrimalNodeIndex>, Node<B::PrimalNodeIndex, B::Scalar>>>,
    pub edges: Arc<
        BTreeMap<EdgeIndex<NavmeshIndex<B::PrimalNodeIndex>>, (Edge<B::PrimalNodeIndex>, usize)>,
    >,
    pub edge_paths: Box<[EdgePaths<B::EtchedPath, B::GapComment>]>,
}

pub struct NavmeshRef<'a, B: NavmeshBase> {
    pub nodes: &'a BTreeMap<NavmeshIndex<B::PrimalNodeIndex>, Node<B::PrimalNodeIndex, B::Scalar>>,
    pub edges: &'a BTreeMap<
        EdgeIndex<NavmeshIndex<B::PrimalNodeIndex>>,
        (Edge<B::PrimalNodeIndex>, usize),
    >,
    pub edge_paths: &'a [EdgePaths<B::EtchedPath, B::GapComment>],
}

pub struct NavmeshRefMut<'a, B: NavmeshBase> {
    pub nodes: &'a BTreeMap<NavmeshIndex<B::PrimalNodeIndex>, Node<B::PrimalNodeIndex, B::Scalar>>,
    pub edges: &'a BTreeMap<
        EdgeIndex<NavmeshIndex<B::PrimalNodeIndex>>,
        (Edge<B::PrimalNodeIndex>, usize),
    >,
    pub edge_paths: &'a mut [EdgePaths<B::EtchedPath, B::GapComment>],
}

impl<B: NavmeshBase> Clone for NavmeshRef<'_, B> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: NavmeshBase> Copy for NavmeshRef<'_, B> {}

impl<B: NavmeshBase> Default for Navmesh<B> {
    fn default() -> Self {
        Self {
            nodes: Arc::new(BTreeMap::new()),
            edges: Arc::new(BTreeMap::new()),
            edge_paths: Vec::new().into_boxed_slice(),
        }
    }
}

impl<B: NavmeshBase> NavmeshBase for NavmeshSer<B> {
    type PrimalNodeIndex = B::PrimalNodeIndex;
    type EtchedPath = B::EtchedPath;
    type GapComment = B::GapComment;
    type Scalar = B::Scalar;
}

impl<B: NavmeshBase> NavmeshBase for Navmesh<B> {
    type PrimalNodeIndex = B::PrimalNodeIndex;
    type EtchedPath = B::EtchedPath;
    type GapComment = B::GapComment;
    type Scalar = B::Scalar;
}

impl<B: NavmeshBase> NavmeshBase for NavmeshRef<'_, B> {
    type PrimalNodeIndex = B::PrimalNodeIndex;
    type EtchedPath = B::EtchedPath;
    type GapComment = B::GapComment;
    type Scalar = B::Scalar;
}

impl<B: NavmeshBase> NavmeshBase for NavmeshRefMut<'_, B> {
    type PrimalNodeIndex = B::PrimalNodeIndex;
    type EtchedPath = B::EtchedPath;
    type GapComment = B::GapComment;
    type Scalar = B::Scalar;
}

impl<B: NavmeshBase> From<NavmeshSer<B>> for Navmesh<B> {
    fn from(this: NavmeshSer<B>) -> Navmesh<B> {
        let mut edge_paths = Vec::with_capacity(this.edges.len());

        let edges = this
            .edges
            .into_iter()
            .map(|(key, (edge, paths))| {
                let idx = edge_paths.len();
                edge_paths.push(paths);
                (key, (edge, idx))
            })
            .collect::<BTreeMap<_, _>>();

        Self {
            nodes: this.nodes,
            edges: Arc::new(edges),
            edge_paths: edge_paths.into_boxed_slice(),
        }
    }
}

impl<B: NavmeshBase> From<Navmesh<B>> for NavmeshSer<B> {
    fn from(this: Navmesh<B>) -> NavmeshSer<B> {
        Self {
            nodes: this.nodes,
            edges: this
                .edges
                .iter()
                .map(|(key, (value, idx))| {
                    (key.clone(), (value.clone(), this.edge_paths[*idx].clone()))
                })
                .collect(),
        }
    }
}

impl<B: NavmeshBase> Navmesh<B> {
    #[inline(always)]
    pub fn as_ref(&self) -> NavmeshRef<B> {
        NavmeshRef {
            nodes: &self.nodes,
            edges: &self.edges,
            edge_paths: &self.edge_paths,
        }
    }

    #[inline(always)]
    pub fn as_mut(&mut self) -> NavmeshRefMut<B> {
        NavmeshRefMut {
            nodes: &self.nodes,
            edges: &self.edges,
            edge_paths: &mut self.edge_paths,
        }
    }
}

pub(crate) fn resolve_edge_data<PNI: Ord, EP>(
    edges: &BTreeMap<EdgeIndex<NavmeshIndex<PNI>>, (Edge<PNI>, usize)>,
    from_node: NavmeshIndex<PNI>,
    to_node: NavmeshIndex<PNI>,
) -> Option<(Edge<&PNI>, MaybeReversed<usize, EP>)> {
    let reversed = from_node > to_node;
    let edge_idx: EdgeIndex<NavmeshIndex<PNI>> = (from_node, to_node).into();
    let edge = edges.get(&edge_idx)?;
    let mut data = edge.0.as_ref();
    if reversed {
        data.flip();
    }
    let mut ret = MaybeReversed::new(edge.1);
    ret.reversed = reversed;
    Some((data, ret))
}

/// Removes a path (weak or normal) with the given label from the navmesh
pub fn remove_path<EP, GC>(edge_paths: &mut [EdgePaths<EP, GC>], label: &RelaxedPath<EP, GC>)
where
    EP: Clone + Eq,
    GC: Clone + PartialEq,
{
    let mut updated_indices = Vec::new();
    for i in edge_paths {
        updated_indices.clear();
        updated_indices.extend(i[..].iter().filter(|&j| j != label).cloned());

        if updated_indices.len() != i.len() {
            *i = take(&mut updated_indices).into_boxed_slice().into();
        }
    }
}

impl<'a, B: NavmeshBase + 'a> NavmeshRefMut<'a, B> {
    #[inline(always)]
    pub fn as_ref(&'a self) -> NavmeshRef<'a, B> {
        NavmeshRef {
            nodes: self.nodes,
            edges: self.edges,
            edge_paths: self.edge_paths,
        }
    }

    pub fn edge_data_mut(
        &mut self,
        from_node: NavmeshIndex<B::PrimalNodeIndex>,
        to_node: NavmeshIndex<B::PrimalNodeIndex>,
    ) -> Option<
        MaybeReversed<
            &mut Arc<[RelaxedPath<B::EtchedPath, B::GapComment>]>,
            RelaxedPath<B::EtchedPath, B::GapComment>,
        >,
    > {
        self.resolve_edge_data(from_node, to_node)
            .map(|(_, item)| item)
            .map(|item| self.access_edge_paths_mut(item))
    }

    #[inline(always)]
    pub fn resolve_edge_data(
        &self,
        from_node: NavmeshIndex<B::PrimalNodeIndex>,
        to_node: NavmeshIndex<B::PrimalNodeIndex>,
    ) -> Option<(
        Edge<&B::PrimalNodeIndex>,
        MaybeReversed<usize, RelaxedPath<B::EtchedPath, B::GapComment>>,
    )> {
        resolve_edge_data(self.edges, from_node, to_node)
    }

    /// Removes a path (weak or normal) with the given label from the navmesh
    pub fn remove_path(&mut self, label: &RelaxedPath<B::EtchedPath, B::GapComment>)
    where
        B::GapComment: PartialEq,
    {
        remove_path(&mut self.edge_paths[..], label);
    }

    /// ## Panics
    /// This function panics if the given `item` is out of bounds
    /// (e.g. only happens when it was produced by a `Navmesh` with different edges)
    #[inline(always)]
    pub fn access_edge_paths_mut(
        &mut self,
        item: MaybeReversed<usize, RelaxedPath<B::EtchedPath, B::GapComment>>,
    ) -> MaybeReversed<
        &mut Arc<[RelaxedPath<B::EtchedPath, B::GapComment>]>,
        RelaxedPath<B::EtchedPath, B::GapComment>,
    > {
        crate::mayrev::index_mut_forward(self.edge_paths, item)
    }
}

impl<'a, B: NavmeshBase + 'a> NavmeshRef<'a, B> {
    #[inline(always)]
    pub fn resolve_edge_data(
        &self,
        from_node: NavmeshIndex<B::PrimalNodeIndex>,
        to_node: NavmeshIndex<B::PrimalNodeIndex>,
    ) -> Option<(
        Edge<&B::PrimalNodeIndex>,
        MaybeReversed<usize, RelaxedPath<B::EtchedPath, B::GapComment>>,
    )> {
        resolve_edge_data(self.edges, from_node, to_node)
    }

    #[inline(always)]
    pub fn node_data(
        &self,
        node: &NavmeshIndex<B::PrimalNodeIndex>,
    ) -> Option<&'a Node<B::PrimalNodeIndex, B::Scalar>> {
        self.nodes.get(node)
    }

    pub fn edge_data(
        &self,
        from_node: NavmeshIndex<B::PrimalNodeIndex>,
        to_node: NavmeshIndex<B::PrimalNodeIndex>,
    ) -> Option<
        MaybeReversed<
            &'a Arc<[RelaxedPath<B::EtchedPath, B::GapComment>]>,
            RelaxedPath<B::EtchedPath, B::GapComment>,
        >,
    > {
        self.resolve_edge_data(from_node, to_node)
            .map(|(_, item)| self.access_edge_paths(item))
    }

    /*
    pub fn check_edge_rules<'b, ObjectKind: Copy + Eq + Ord>(
        &self,
        from_node: NavmeshIndex<B::PrimalNodeIndex>,
        to_node: NavmeshIndex<B::PrimalNodeIndex>,
    ) -> Option<(B::Scalar, bool)>
    where
        for<'i> &'i B::PrimalNodeIndex: topola_rules::GetConditions<'i, ObjectKind = ObjectKind>,
        for<'i> &'i B::EtchedPath: topola_rules::GetConditions<'i, ObjectKind = ObjectKind>,
        B::EtchedPath: topola_rules::GetWidth<Scalar = B::Scalar>,
        R: topola_rules::AccessRules<Scalar = B::Scalar, ObjectKind = ObjectKind>,
        B::Scalar: Default + num_traits::Float + core::iter::Sum,
    {
        let edgd = self.edge_data(from_node.clone(), to_node.clone())?;
        if edgd.inner.is_empty() {
            return Some((B::Scalar::default(), true));
        }
        let (NavmeshIndex::Dual(_), NavmeshIndex::Dual(_)) = (from_node.clone(), to_node.clone())
        else {
            // we only check dual-dual connections for now
            return Some((B::Scalar::default(), true));
        };
        // decode dual nodes into the unique orthogonal primal nodes
        let start_node_data = self.node_data(&from_node)?;
        let start_node_neighs = &start_node_data.neighs;

        let start_target_pos = start_node_neighs
            .iter()
            .enumerate()
            .find(|(_, i)| **i == from_node)?
            .0;
        let prev = &start_node_neighs
            [(start_node_neighs.len() + start_target_pos - 1) % start_node_neighs.len()];
        let next = &start_node_neighs[(start_target_pos + 1) % start_node_neighs.len()];

        let filtered_edgd: Vec<_> = Iterator::filter_map(edgd.iter(), |i| match i {
            RelaxedPath::Normal(i) => Some(i),
            RelaxedPath::Weak(_) => None,
        })
        .collect();

        let usage_width: B::Scalar =
            Iterator::map(filtered_edgd.iter(), topola_rules::GetWidth::width).sum();

        let conds: Vec<_> = filtered_edgd
            .iter()
            .filter_map(|i| i.conditions())
            .collect();
        let mut usage = usage_width
            + conds
                .windows(2)
                .filter_map(|x| if let [i, j] = x { Some((i, j)) } else { None })
                .map(|(i, j)| self.rules.clearance((i, j).into()))
                .sum();

        let (NavmeshIndex::Primal(prev), NavmeshIndex::Primal(next)) = (prev, next) else {
            return if let (
                NavmeshIndex::Dual(DualIndex::Outer(_)),
                NavmeshIndex::Dual(DualIndex::Outer(_)),
            ) = (from_node, to_node)
            {
                // `DualOuter`-`DualOuter` are only associated to a single primal
                // TODO: but even those should have a maximum capacity (e.g. up to PCB border)
                // TODO: handle (more difficult) how large polygons are and such...
                Some((usage, true))
            } else {
                // something is wrong
                None
            };
        };

        let stop_node_data = self.node_data(&to_node)?;
        let to_primals: BTreeSet<_> = stop_node_data.primal_neighbors().collect();
        assert!(to_primals.contains(&prev));
        assert!(to_primals.contains(&next));

        use topola_rules::GetConditions;
        let prev_cond = prev.conditions();
        let next_cond = next.conditions();

        if let Some(prev_cond) = prev_cond {
            usage = usage
                + self
                    .rules
                    .clearance((&prev_cond, conds.first().unwrap()).into());
        }

        if let Some(next_cond) = next_cond {
            usage = usage
                + self
                    .rules
                    .clearance((&next_cond, conds.last().unwrap()).into());
        }

        // TODO: handle (more difficult) how large polygons are and such...
        Some((
            usage,
            usage
                <= B::Scalar::hypot(
                    stop_node_data.pos.x - start_node_data.pos.x,
                    stop_node_data.pos.y - start_node_data.pos.y,
                ),
        ))
    }
    */

    /// ## Panics
    /// This function panics if the given `item` is out of bounds
    /// (e.g. only happens when it was produced by a `Navmesh` with different edges)
    #[inline(always)]
    pub fn access_edge_paths(
        &self,
        item: MaybeReversed<usize, RelaxedPath<B::EtchedPath, B::GapComment>>,
    ) -> MaybeReversed<
        &'a Arc<[RelaxedPath<B::EtchedPath, B::GapComment>]>,
        RelaxedPath<B::EtchedPath, B::GapComment>,
    > {
        crate::mayrev::index_forward(self.edge_paths, item)
    }

    /// See [`find_other_end`](planarr::find_other_end).
    pub fn planarr_find_other_end(
        self,
        node: &NavmeshIndex<B::PrimalNodeIndex>,
        start: &NavmeshIndex<B::PrimalNodeIndex>,
        pos: usize,
        already_inserted_at_start: bool,
        stop: &NavmeshIndex<B::PrimalNodeIndex>,
    ) -> Option<(usize, planarr::OtherEnd)> {
        planarr::find_other_end(
            self.nodes[node].neighs.iter().map(move |neigh| {
                let edge = self
                    .edge_data(node.clone(), neigh.clone())
                    .expect("unable to resolve neighbor");
                (neigh.clone(), edge)
            }),
            start,
            pos,
            already_inserted_at_start,
            stop,
        )
    }

    /// See [`find_all_other_ends`](planarr::find_all_other_ends).
    pub fn planarr_find_all_other_ends(
        self,
        node: &'a NavmeshIndex<B::PrimalNodeIndex>,
        start: &'a NavmeshIndex<B::PrimalNodeIndex>,
        pos: usize,
        already_inserted_at_start: bool,
    ) -> Option<(
        usize,
        impl Iterator<Item = (NavmeshIndex<B::PrimalNodeIndex>, planarr::OtherEnd)> + 'a,
    )> {
        planarr::find_all_other_ends(
            self.nodes[node].neighs.iter().map(move |neigh| {
                let edge = self
                    .edge_data(node.clone(), neigh.clone())
                    .expect("unable to resolve neighbor");
                (neigh.clone(), edge)
            }),
            start,
            pos,
            already_inserted_at_start,
        )
    }
}
