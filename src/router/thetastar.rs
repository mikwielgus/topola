// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT
//
// This file was originally copied from astar.rs of library Petgraph at
// https://github.com/petgraph/petgraph/blob/master/src/algo/astar.rs
//
// Petgraph's original copyright line is
// Copyright (c) 2015

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BinaryHeap};

use std::ops::{ControlFlow, Sub};

use derive_getters::Getters;
use petgraph::algo::Measure;
use petgraph::visit::{EdgeRef, GraphBase, IntoEdgeReferences, IntoEdges};
use thiserror::Error;

use std::cmp::Ordering;

use crate::stepper::{EstimateProgress, Step};

#[derive(Copy, Clone, Debug)]
pub struct MinScored<K, T>(pub K, pub T);

impl<K: PartialOrd, T> PartialEq for MinScored<K, T> {
    #[inline]
    fn eq(&self, other: &MinScored<K, T>) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl<K: PartialOrd, T> Eq for MinScored<K, T> {}

impl<K: PartialOrd, T> PartialOrd for MinScored<K, T> {
    #[inline]
    fn partial_cmp(&self, other: &MinScored<K, T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<K: PartialOrd, T> Ord for MinScored<K, T> {
    #[inline]
    fn cmp(&self, other: &MinScored<K, T>) -> Ordering {
        let a = &self.0;
        let b = &other.0;
        if a == b {
            Ordering::Equal
        } else if a < b {
            Ordering::Greater
        } else if a > b {
            Ordering::Less
        } else if a.ne(a) && b.ne(b) {
            // these are the NaN cases
            Ordering::Equal
        } else if a.ne(a) {
            // Order NaN less, so that it is last in the MinScore order
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

#[derive(Debug)]
pub struct PathTracker<G>
where
    G: GraphBase,
    G::NodeId: Eq + Ord,
{
    predecessors: BTreeMap<G::NodeId, G::NodeId>,
}

impl<G> PathTracker<G>
where
    G: GraphBase,
    G::NodeId: Eq + Ord,
{
    fn new() -> PathTracker<G> {
        PathTracker {
            predecessors: BTreeMap::new(),
        }
    }

    fn predecessor(&self, node: G::NodeId) -> Option<G::NodeId> {
        self.predecessors.get(&node).copied()
    }

    fn set_predecessor(&mut self, node: G::NodeId, previous: G::NodeId) {
        self.predecessors.insert(node, previous);
    }

    pub fn reconstruct_path_to(&self, last: G::NodeId) -> Vec<G::NodeId> {
        let mut path = vec![last];

        let mut current = last;
        while let Some(&previous) = self.predecessors.get(&current) {
            path.push(previous);
            current = previous;
        }

        path.reverse();

        path
    }
}

pub trait ThetastarStrategy<G, K, R>
where
    G: GraphBase,
    G::NodeId: Eq + Ord,
    for<'a> &'a G: IntoEdges<NodeId = G::NodeId, EdgeId = G::EdgeId> + MakeEdgeRef,
    K: Measure + Copy,
{
    fn visit_navnode(
        &mut self,
        graph: &G,
        navnode: G::NodeId,
        tracker: &PathTracker<G>,
    ) -> Result<Option<R>, ()>;
    fn place_probe_to_navnode(&mut self, graph: &G, probed_navnode: G::NodeId) -> Option<K>;
    fn remove_probe(&mut self, graph: &G);
    fn estimate_cost_to_goal(&mut self, graph: &G, navnode: G::NodeId) -> K;
}

pub trait MakeEdgeRef: IntoEdgeReferences {
    fn edge_ref(&self, edge_id: Self::EdgeId) -> Self::EdgeRef;
}

/// The enum that describes the current state of the algorithm.
#[derive(Clone, Copy, Debug)]
pub enum ThetastarState<N: Copy, E: Copy> {
    /// Visit a new navnode and copy all navedges going out of it.
    Scanning,
    /// Load a new navedge, try to probe it from current navnode's predecessor.
    /// If this fails, transition to `VisitingProbeOnNavedge`. If there is no
    /// more navedges, transition back to `Scanning`.
    VisitingProbeOnLineOfSight(N),
    /// Probe the currently loaded navedge from the current navnode.
    VisitingProbeOnNavedge(N, E),
    /// The probe is in place, retract it and continue to the next navedge.
    Probing(N),
}

/// The pathfinding algorithm Topola uses to find the shortest path to route
/// is Theta*. Theta* is just A* with an improvement: every time an navedge is
/// scanned, the algorithm first tries to draw directly to its target navnode
/// from the predecessor of the currently visited navnode. Note that this
/// creates paths with edges that do not all lie on the navmesh.
#[derive(Getters)]
pub struct ThetastarStepper<G, K>
where
    G: GraphBase,
    G::NodeId: Eq + Ord,
    for<'a> &'a G: IntoEdges<NodeId = G::NodeId, EdgeId = G::EdgeId> + MakeEdgeRef,
    K: Measure + Copy + Sub<Output = K>,
{
    state: ThetastarState<G::NodeId, G::EdgeId>,
    graph: G,
    /// The priority queue of the navnodes to expand.
    #[getter(skip)]
    frontier: BinaryHeap<MinScored<K, G::NodeId>>,
    /// Also known as the g-scores, or just g.
    scores: BTreeMap<G::NodeId, K>,
    /// Also known as the f-scores, or just f.
    cost_to_goal_estimate_scores: BTreeMap<G::NodeId, K>,
    #[getter(skip)]
    path_tracker: PathTracker<G>,
    // FIXME: To work around edge references borrowing from the graph we collect then reiterate over them.
    #[getter(skip)]
    edge_ids: Vec<G::EdgeId>,

    #[getter(skip)]
    progress_estimate_value: K,

    #[getter(skip)]
    progress_estimate_maximum: K,
}

#[derive(Error, Debug, Clone)]
pub enum ThetastarError {
    #[error("A* search found no path")]
    NotFound,
}

impl<G, K> ThetastarStepper<G, K>
where
    G: GraphBase,
    G::NodeId: Eq + Ord,
    for<'a> &'a G: IntoEdges<NodeId = G::NodeId, EdgeId = G::EdgeId> + MakeEdgeRef,
    K: Measure + Copy + Sub<Output = K>,
{
    pub fn new<R>(
        graph: G,
        start: G::NodeId,
        strategy: &mut impl ThetastarStrategy<G, K, R>,
    ) -> Self {
        let estimated_cost_from_start_to_goal = strategy.estimate_cost_to_goal(&graph, start);

        let mut this = Self {
            state: ThetastarState::Scanning,
            graph,
            frontier: BinaryHeap::new(),
            scores: BTreeMap::new(),
            cost_to_goal_estimate_scores: BTreeMap::new(),
            path_tracker: PathTracker::<G>::new(),
            edge_ids: Vec::new(),
            progress_estimate_value: K::default(),
            progress_estimate_maximum: estimated_cost_from_start_to_goal,
        };

        let zero_score = K::default();
        this.scores.insert(start, zero_score);
        this.frontier
            .push(MinScored(estimated_cost_from_start_to_goal, start));
        this
    }

    fn push_to_frontier<R>(
        &mut self,
        next: G::NodeId,
        next_score: K,
        predecessor: G::NodeId,
        strategy: &mut impl ThetastarStrategy<G, K, R>,
    ) {
        let cost_to_goal_estimate = strategy.estimate_cost_to_goal(&self.graph, next);

        if cost_to_goal_estimate > self.progress_estimate_maximum {
            self.progress_estimate_maximum = cost_to_goal_estimate;
        }

        if self.progress_estimate_maximum - cost_to_goal_estimate > self.progress_estimate_value {
            self.progress_estimate_value = self.progress_estimate_maximum - cost_to_goal_estimate;
        }

        self.path_tracker.set_predecessor(next, predecessor);
        let next_estimate_score = next_score + cost_to_goal_estimate;
        self.frontier.push(MinScored(next_estimate_score, next));
    }
}

impl<G, K, R, S: ThetastarStrategy<G, K, R>>
    Step<S, (K, Vec<G::NodeId>, R), ThetastarState<G::NodeId, G::EdgeId>> for ThetastarStepper<G, K>
where
    G: GraphBase,
    G::NodeId: Eq + Ord,
    for<'a> &'a G: IntoEdges<NodeId = G::NodeId, EdgeId = G::EdgeId> + MakeEdgeRef,
    K: Measure + Copy + Sub<Output = K>,
{
    type Error = ThetastarError;

    fn step(
        &mut self,
        strategy: &mut S,
    ) -> Result<
        ControlFlow<(K, Vec<G::NodeId>, R), ThetastarState<G::NodeId, G::EdgeId>>,
        ThetastarError,
    > {
        match self.state {
            ThetastarState::Scanning => {
                let Some(MinScored(estimate_score, navnode)) = self.frontier.pop() else {
                    return Err(ThetastarError::NotFound);
                };

                let Ok(maybe_result) =
                    strategy.visit_navnode(&self.graph, navnode, &self.path_tracker)
                else {
                    return Ok(ControlFlow::Continue(self.state));
                };

                if let Some(result) = maybe_result {
                    let path = self.path_tracker.reconstruct_path_to(navnode);
                    let cost = self.scores[&navnode];
                    return Ok(ControlFlow::Break((cost, path, result)));
                }

                match self.cost_to_goal_estimate_scores.entry(navnode) {
                    Entry::Occupied(mut entry) => {
                        // If the node has already been visited with an equal or lower
                        // estimated score than now, then we do not need to re-visit it.
                        if *entry.get() <= estimate_score {
                            return Ok(ControlFlow::Continue(self.state));
                        }
                        entry.insert(estimate_score);
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(estimate_score);
                    }
                }

                self.edge_ids = self.graph.edges(navnode).map(|edge| edge.id()).collect();

                self.state = ThetastarState::VisitingProbeOnLineOfSight(navnode);
                Ok(ControlFlow::Continue(self.state))
            }
            ThetastarState::VisitingProbeOnLineOfSight(visited_navnode) => {
                if let Some(curr_navedge) = self.edge_ids.pop() {
                    // This lookup can be unwrapped without fear of panic since the node was
                    // necessarily scored before adding it to `.visit_next`.
                    //let node_score = self.scores[&visited_navnode];
                    let to_navnode = (&self.graph).edge_ref(curr_navedge).target();

                    if let Some(parent_navnode) = self.path_tracker.predecessor(visited_navnode) {
                        // Visit parent node.
                        strategy.visit_navnode(&self.graph, parent_navnode, &self.path_tracker);

                        let parent_score = self.scores[&parent_navnode];

                        if let Some(los_cost) =
                            strategy.place_probe_to_navnode(&self.graph, to_navnode)
                        {
                            let next = to_navnode;
                            let next_score = parent_score + los_cost;

                            match self.scores.entry(next) {
                                Entry::Occupied(mut entry) => {
                                    // No need to add neighbors that we have already reached through a
                                    // shorter path than now.
                                    if *entry.get() <= next_score {
                                        self.state = ThetastarState::VisitingProbeOnNavedge(
                                            visited_navnode,
                                            curr_navedge,
                                        );
                                        return Ok(ControlFlow::Continue(self.state));
                                    }
                                    entry.insert(next_score);
                                }
                                Entry::Vacant(entry) => {
                                    entry.insert(next_score);
                                }
                            }

                            self.push_to_frontier(next, next_score, parent_navnode, strategy);

                            self.state = ThetastarState::Probing(visited_navnode);
                            Ok(ControlFlow::Continue(self.state))
                        } else {
                            // Come back from parent node if drawing from it failed.
                            strategy.visit_navnode(
                                &self.graph,
                                visited_navnode,
                                &self.path_tracker,
                            );
                            self.state = ThetastarState::VisitingProbeOnNavedge(
                                visited_navnode,
                                curr_navedge,
                            );
                            Ok(ControlFlow::Continue(self.state))
                        }
                    } else {
                        self.state =
                            ThetastarState::VisitingProbeOnNavedge(visited_navnode, curr_navedge);
                        Ok(ControlFlow::Continue(self.state))
                    }
                } else {
                    self.state = ThetastarState::Scanning;
                    Ok(ControlFlow::Continue(self.state))
                }
            }
            ThetastarState::VisitingProbeOnNavedge(visited_navnode, curr_navedge) => {
                let visited_score = self.scores[&visited_navnode];
                let to_navnode = (&self.graph).edge_ref(curr_navedge).target();

                if let Some(navedge_cost) = strategy.place_probe_to_navnode(&self.graph, to_navnode)
                {
                    let next = to_navnode;
                    let next_score = visited_score + navedge_cost;

                    match self.scores.entry(next) {
                        Entry::Occupied(mut entry) => {
                            // No need to add neighbors that we have already reached through a
                            // shorter path than now.
                            if *entry.get() <= next_score {
                                self.state = ThetastarState::Probing(visited_navnode);
                                return Ok(ControlFlow::Continue(self.state));
                            }
                            entry.insert(next_score);
                        }
                        Entry::Vacant(entry) => {
                            entry.insert(next_score);
                        }
                    }

                    self.push_to_frontier(next, next_score, visited_navnode, strategy);

                    self.state = ThetastarState::Probing(visited_navnode);
                    Ok(ControlFlow::Continue(self.state))
                } else {
                    self.state = ThetastarState::VisitingProbeOnLineOfSight(visited_navnode);
                    Ok(ControlFlow::Continue(self.state))
                }
            }
            ThetastarState::Probing(visited_navnode) => {
                strategy.remove_probe(&self.graph);

                self.state = ThetastarState::VisitingProbeOnLineOfSight(visited_navnode);
                Ok(ControlFlow::Continue(self.state))
            }
        }
    }
}

impl<G, K> EstimateProgress for ThetastarStepper<G, K>
where
    G: GraphBase,
    G::NodeId: Eq + Ord,
    for<'a> &'a G: IntoEdges<NodeId = G::NodeId, EdgeId = G::EdgeId> + MakeEdgeRef,
    K: Measure + Copy + Sub<Output = K>,
{
    type Value = K;

    fn estimate_progress_value(&self) -> K {
        self.progress_estimate_value
    }

    fn estimate_progress_maximum(&self) -> K {
        self.progress_estimate_maximum
    }
}
