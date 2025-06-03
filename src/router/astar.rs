// Copyright (c) 2015
// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::collections::{btree_map::Entry, BTreeMap, BinaryHeap, VecDeque};

use std::ops::ControlFlow;

use derive_getters::Getters;
use petgraph::algo::Measure;
use petgraph::visit::{EdgeRef, GraphBase, IntoEdgeReferences, IntoEdges};
use thiserror::Error;

use std::cmp::Ordering;

use crate::stepper::Step;

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
    came_from: BTreeMap<G::NodeId, G::NodeId>,
}

impl<G> PathTracker<G>
where
    G: GraphBase,
    G::NodeId: Eq + Ord,
{
    fn new() -> PathTracker<G> {
        PathTracker {
            came_from: BTreeMap::new(),
        }
    }

    fn set_predecessor(&mut self, node: G::NodeId, previous: G::NodeId) {
        self.came_from.insert(node, previous);
    }

    pub fn reconstruct_path_to(&self, last: G::NodeId) -> Vec<G::NodeId> {
        let mut path = vec![last];

        let mut current = last;
        while let Some(&previous) = self.came_from.get(&current) {
            path.push(previous);
            current = previous;
        }

        path.reverse();

        path
    }
}

pub trait AstarStrategy<G, K, R>
where
    G: GraphBase,
    G::NodeId: Eq + Ord,
    for<'a> &'a G: IntoEdges<NodeId = G::NodeId, EdgeId = G::EdgeId> + MakeEdgeRef,
    K: Measure + Copy,
{
    fn visit_navnode(
        &mut self,
        graph: &G,
        node: G::NodeId,
        tracker: &PathTracker<G>,
    ) -> Result<Option<R>, ()>;
    fn place_probe_at_navedge<'a>(
        &mut self,
        graph: &'a G,
        edge: <&'a G as IntoEdgeReferences>::EdgeRef,
    ) -> Option<K>;
    fn remove_probe(&mut self, graph: &G);
    fn estimate_cost(&mut self, graph: &G, node: G::NodeId) -> K;
}

pub trait MakeEdgeRef: IntoEdgeReferences {
    fn edge_ref(&self, edge_id: Self::EdgeId) -> Self::EdgeRef;
}

#[derive(Getters)]
pub struct AstarStepper<G, K>
where
    G: GraphBase,
    G::NodeId: Eq + Ord,
    for<'a> &'a G: IntoEdges<NodeId = G::NodeId, EdgeId = G::EdgeId> + MakeEdgeRef,
    K: Measure + Copy,
{
    graph: G,
    #[getter(skip)]
    visit_next: BinaryHeap<MinScored<K, G::NodeId>>,
    /// Also known as the g-scores, or just g.
    scores: BTreeMap<G::NodeId, K>,
    /// Also known as the f-scores, or just f.
    estimate_scores: BTreeMap<G::NodeId, K>,
    #[getter(skip)]
    path_tracker: PathTracker<G>,
    #[getter(skip)]
    maybe_curr_node: Option<G::NodeId>,
    // FIXME: To work around edge references borrowing from the graph we collect then reiterate over them.
    #[getter(skip)]
    edge_ids: VecDeque<G::EdgeId>,
    // TODO: Rewrite this to be a well-designed state machine. Booleans are a code smell.
    #[getter(skip)]
    is_probing: bool,
}

/// The status enum of the A* stepper returned when there is no failure or
/// break.
///
/// Note that, when thinking of the A* stepper as of a state machine, the
/// variants of the status actually correspond to state transitions, not to
/// states themselves, since `Probing` and `ProbingButDiscarded`, and likewise
/// `VisitSkipped` and `Visited`, would correspond to the same state.
#[derive(Debug)]
pub enum AstarContinueStatus {
    /// A* has now placed a probe to measure the cost of the edge to a
    /// neighboring navnode from the current position. The probed navnode has
    /// been added to the priority queue, and the newly measured edge cost has
    /// been stored in a map.
    Probing,
    /// A* has now placed a probe, but it turned out that the probed navnode has
    /// been previously reached through a path with equal or lower score, so the
    /// probe's measurement has been discarded. The probe, however, will be only
    /// removed in the next state just as if it was after the normal `Probing`
    /// status.
    ProbingButDiscarded,
    /// The probe that had been placed in the previous state has now been
    /// removed.
    ///
    /// The probe is only removed in this separate state to make it possible
    /// to pause the A* while the placed probe exists, which is very useful
    /// for debugging.
    Probed,
    /// A* has now attempted to visit a new navnode, but it turned out that
    /// it has been previously reached through a path with an equal or lower
    /// estimated score, so the visit to that navnode has been skipped.
    VisitSkipped,
    /// A* has failed to visit a new navnode. Happens, so A* will just proceed
    /// to the next node in the priority queue.
    VisitFailed,
    /// A* has now visited a new navnode.
    ///
    /// Quick recap if you have been trying to remember what is the difference
    /// between probing and visiting: probing is done as part of a scan of
    /// neighboring navnodes around the currently visited navnode to add them to
    /// the priority queue, whereas when a navnode is visited it is taken from
    /// the priority queue to actually become the currently visited navnode.
    Visited,
}

#[derive(Error, Debug, Clone)]
pub enum AstarError {
    #[error("A* search found no path")]
    NotFound,
}

impl<G, K> AstarStepper<G, K>
where
    G: GraphBase,
    G::NodeId: Eq + Ord,
    for<'a> &'a G: IntoEdges<NodeId = G::NodeId, EdgeId = G::EdgeId> + MakeEdgeRef,
    K: Measure + Copy,
{
    pub fn new<R>(graph: G, start: G::NodeId, strategy: &mut impl AstarStrategy<G, K, R>) -> Self {
        let mut this = Self {
            graph,
            visit_next: BinaryHeap::new(),
            scores: BTreeMap::new(),
            estimate_scores: BTreeMap::new(),
            path_tracker: PathTracker::<G>::new(),
            maybe_curr_node: None,
            edge_ids: VecDeque::new(),
            is_probing: false,
        };

        let zero_score = K::default();
        this.scores.insert(start, zero_score);
        this.visit_next
            .push(MinScored(strategy.estimate_cost(&this.graph, start), start));
        this
    }
}

impl<G, K, R, S: AstarStrategy<G, K, R>> Step<S, (K, Vec<G::NodeId>, R), AstarContinueStatus>
    for AstarStepper<G, K>
where
    G: GraphBase,
    G::NodeId: Eq + Ord,
    for<'a> &'a G: IntoEdges<NodeId = G::NodeId, EdgeId = G::EdgeId> + MakeEdgeRef,
    K: Measure + Copy,
{
    type Error = AstarError;

    fn step(
        &mut self,
        strategy: &mut S,
    ) -> Result<ControlFlow<(K, Vec<G::NodeId>, R), AstarContinueStatus>, AstarError> {
        if let Some(curr_node) = self.maybe_curr_node {
            if self.is_probing {
                strategy.remove_probe(&self.graph);
                self.is_probing = false;
            }

            if let Some(edge_id) = self.edge_ids.pop_front() {
                // This lookup can be unwrapped without fear of panic since the node was
                // necessarily scored before adding it to `.visit_next`.
                let node_score = self.scores[&curr_node];
                let edge = (&self.graph).edge_ref(edge_id);

                if let Some(edge_cost) = strategy.place_probe_at_navedge(&self.graph, edge) {
                    let next = edge.target();
                    let next_score = node_score + edge_cost;

                    match self.scores.entry(next) {
                        Entry::Occupied(mut entry) => {
                            // No need to add neighbors that we have already reached through a
                            // shorter path than now.
                            if *entry.get() <= next_score {
                                return Ok(ControlFlow::Continue(
                                    AstarContinueStatus::ProbingButDiscarded,
                                ));
                            }
                            entry.insert(next_score);
                        }
                        Entry::Vacant(entry) => {
                            entry.insert(next_score);
                        }
                    }

                    self.path_tracker.set_predecessor(next, curr_node);
                    let next_estimate_score =
                        next_score + strategy.estimate_cost(&self.graph, next);
                    self.visit_next.push(MinScored(next_estimate_score, next));

                    self.is_probing = true;
                    return Ok(ControlFlow::Continue(AstarContinueStatus::Probing));
                }

                return Ok(ControlFlow::Continue(AstarContinueStatus::Probed));
            }

            self.maybe_curr_node = None;
        }

        let Some(MinScored(estimate_score, node)) = self.visit_next.pop() else {
            return Err(AstarError::NotFound);
        };

        let Ok(maybe_result) = strategy.visit_navnode(&self.graph, node, &self.path_tracker) else {
            return Ok(ControlFlow::Continue(AstarContinueStatus::VisitFailed));
        };

        if let Some(result) = maybe_result {
            let path = self.path_tracker.reconstruct_path_to(node);
            let cost = self.scores[&node];
            return Ok(ControlFlow::Break((cost, path, result)));
        }

        match self.estimate_scores.entry(node) {
            Entry::Occupied(mut entry) => {
                // If the node has already been visited with an equal or lower
                // estimated score than now, then we do not need to re-visit it.
                if *entry.get() <= estimate_score {
                    return Ok(ControlFlow::Continue(AstarContinueStatus::VisitSkipped));
                }
                entry.insert(estimate_score);
            }
            Entry::Vacant(entry) => {
                entry.insert(estimate_score);
            }
        }

        self.maybe_curr_node = Some(node);
        self.edge_ids = self.graph.edges(node).map(|edge| edge.id()).collect();

        Ok(ControlFlow::Continue(AstarContinueStatus::Visited))
    }
}
