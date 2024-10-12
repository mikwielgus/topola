/**
 *
 * Copied and substantially modified from petgraph's scored.rs and algo/astar.rs.
 *
 * Copyright (c) 2015
 **/
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::{BinaryHeap, HashMap, VecDeque};

use std::hash::Hash;
use std::ops::ControlFlow;

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
    G::NodeId: Eq + Hash,
{
    came_from: HashMap<G::NodeId, G::NodeId>,
}

impl<G> PathTracker<G>
where
    G: GraphBase,
    G::NodeId: Eq + Hash,
{
    fn new() -> PathTracker<G> {
        PathTracker {
            came_from: HashMap::new(),
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
    G::NodeId: Eq + Hash,
    for<'a> &'a G: IntoEdges<NodeId = G::NodeId, EdgeId = G::EdgeId> + MakeEdgeRef,
    K: Measure + Copy,
{
    fn is_goal(&mut self, graph: &G, node: G::NodeId, tracker: &PathTracker<G>) -> Option<R>;
    fn place_probe<'a>(
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

pub struct Astar<G, K>
where
    G: GraphBase,
    G::NodeId: Eq + Hash,
    for<'a> &'a G: IntoEdges<NodeId = G::NodeId, EdgeId = G::EdgeId> + MakeEdgeRef,
    K: Measure + Copy,
{
    pub graph: G,
    pub visit_next: BinaryHeap<MinScored<K, G::NodeId>>,
    pub scores: HashMap<G::NodeId, K>,
    pub estimate_scores: HashMap<G::NodeId, K>,
    pub path_tracker: PathTracker<G>,
    pub maybe_curr_node: Option<G::NodeId>,
    // FIXME: To work around edge references borrowing from the graph we collect then reiterate over tem.
    pub edge_ids: VecDeque<G::EdgeId>,
    // TODO: Rewrite this to be a well-designed state machine.
    pub is_probing: bool,
}

#[derive(Debug)]
pub enum AstarContinueStatus {
    Probing,
    Probed,
    Visited,
}

#[derive(Error, Debug, Clone)]
pub enum AstarError {
    #[error("A* search found no path")]
    NotFound,
}

impl<G, K> Astar<G, K>
where
    G: GraphBase,
    G::NodeId: Eq + Hash,
    for<'a> &'a G: IntoEdges<NodeId = G::NodeId, EdgeId = G::EdgeId> + MakeEdgeRef,
    K: Measure + Copy,
{
    pub fn new<R>(graph: G, start: G::NodeId, strategy: &mut impl AstarStrategy<G, K, R>) -> Self {
        let mut this = Self {
            graph,
            visit_next: BinaryHeap::new(),
            scores: HashMap::new(),
            estimate_scores: HashMap::new(),
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
    for Astar<G, K>
where
    G: GraphBase,
    G::NodeId: Eq + Hash,
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
                // necessarily scored before adding it to `visit_next`.
                let node_score = self.scores[&curr_node];
                let edge = (&self.graph).edge_ref(edge_id);

                if let Some(edge_cost) = strategy.place_probe(&self.graph, edge) {
                    let next = edge.target();
                    let next_score = node_score + edge_cost;

                    match self.scores.entry(next) {
                        Occupied(mut entry) => {
                            // No need to add neighbors that we have already reached through a
                            // shorter path than now.
                            if *entry.get() <= next_score {
                                return Ok(ControlFlow::Continue(AstarContinueStatus::Probed));
                            }
                            entry.insert(next_score);
                        }
                        Vacant(entry) => {
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

        if let Some(result) = strategy.is_goal(&self.graph, node, &self.path_tracker) {
            let path = self.path_tracker.reconstruct_path_to(node);
            let cost = self.scores[&node];
            return Ok(ControlFlow::Break((cost, path, result)));
        }

        match self.estimate_scores.entry(node) {
            Occupied(mut entry) => {
                // If the node has already been visited with an equal or lower score than
                // now, then we do not need to re-visit it.
                if *entry.get() <= estimate_score {
                    return Ok(ControlFlow::Continue(AstarContinueStatus::Visited));
                }
                entry.insert(estimate_score);
            }
            Vacant(entry) => {
                entry.insert(estimate_score);
            }
        }

        self.maybe_curr_node = Some(node);
        self.edge_ids = self.graph.edges(node).map(|edge| edge.id()).collect();

        Ok(ControlFlow::Continue(AstarContinueStatus::Visited))
    }
}
