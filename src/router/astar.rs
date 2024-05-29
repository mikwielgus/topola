/**
 *
 * Copied and substantially modified from petgraph's scored.rs and algo/astar.rs.
 *
 * Copyright (c) 2015
 **/
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::{BinaryHeap, HashMap};

use std::hash::Hash;

use petgraph::algo::Measure;
use petgraph::visit::{EdgeRef, GraphBase, IntoEdges};
use thiserror::Error;

use std::cmp::Ordering;

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
    G: IntoEdges,
    K: Measure + Copy,
    G::NodeId: Eq + Hash,
{
    fn is_goal(&mut self, node: G::NodeId, tracker: &PathTracker<G>) -> Option<R>;
    fn edge_cost(&mut self, edge: G::EdgeRef) -> Option<K>;
    fn estimate_cost(&mut self, node: G::NodeId) -> K;
}

pub struct Astar<G, K>
where
    G: IntoEdges,
    G::NodeId: Eq + Hash,
    K: Measure + Copy,
{
    pub graph: G,
    pub visit_next: BinaryHeap<MinScored<K, G::NodeId>>,
    pub scores: HashMap<G::NodeId, K>,
    pub estimate_scores: HashMap<G::NodeId, K>,
    pub path_tracker: PathTracker<G>,
}

#[derive(Error, Debug, Clone)]
pub enum AstarError {
    #[error("A* search found no path")]
    NotFound,
}

pub enum AstarStatus<G, K, R>
where
    G: IntoEdges,
    G::NodeId: Eq + Hash,
    K: Measure + Copy,
{
    Running,
    Finished(K, Vec<G::NodeId>, R),
    Error(AstarError),
}

impl<G, K> Astar<G, K>
where
    G: IntoEdges,
    G::NodeId: Eq + Hash,
    K: Measure + Copy,
{
    pub fn new<R>(graph: G, start: G::NodeId, strategy: &mut impl AstarStrategy<G, K, R>) -> Self {
        let mut this = Self {
            graph,
            visit_next: BinaryHeap::new(),
            scores: HashMap::new(),
            estimate_scores: HashMap::new(),
            path_tracker: PathTracker::<G>::new(),
        };

        let zero_score = K::default();
        this.scores.insert(start, zero_score);
        this.visit_next
            .push(MinScored(strategy.estimate_cost(start), start));
        this
    }

    pub fn step<R>(&mut self, strategy: &mut impl AstarStrategy<G, K, R>) -> AstarStatus<G, K, R> {
        let Some(MinScored(estimate_score, node)) = self.visit_next.pop() else {
            return AstarStatus::Error(AstarError::NotFound);
        };

        if let Some(result) = strategy.is_goal(node, &self.path_tracker) {
            let path = self.path_tracker.reconstruct_path_to(node);
            let cost = self.scores[&node];
            return AstarStatus::Finished(cost, path, result);
        }

        // This lookup can be unwrapped without fear of panic since the node was
        // necessarily scored before adding it to `visit_next`.
        let node_score = self.scores[&node];

        match self.estimate_scores.entry(node) {
            Occupied(mut entry) => {
                // If the node has already been visited with an equal or lower score than
                // now, then we do not need to re-visit it.
                if *entry.get() <= estimate_score {
                    return AstarStatus::Running;
                }
                entry.insert(estimate_score);
            }
            Vacant(entry) => {
                entry.insert(estimate_score);
            }
        }

        for edge in self.graph.edges(node) {
            if let Some(edge_cost) = strategy.edge_cost(edge) {
                let next = edge.target();
                let next_score = node_score + edge_cost;

                match self.scores.entry(next) {
                    Occupied(mut entry) => {
                        // No need to add neighbors that we have already reached through a
                        // shorter path than now.
                        if *entry.get() <= next_score {
                            return AstarStatus::Running;
                        }
                        entry.insert(next_score);
                    }
                    Vacant(entry) => {
                        entry.insert(next_score);
                    }
                }

                self.path_tracker.set_predecessor(next, node);
                let next_estimate_score = next_score + strategy.estimate_cost(next);
                self.visit_next.push(MinScored(next_estimate_score, next));
            }
        }

        AstarStatus::Running
    }
}

pub fn astar<G, K, R>(
    graph: G,
    start: G::NodeId,
    strategy: &mut impl AstarStrategy<G, K, R>,
) -> Result<(K, Vec<G::NodeId>, R), AstarError>
where
    G: IntoEdges,
    G::NodeId: Eq + Hash,
    K: Measure + Copy,
{
    let mut astar = Astar::new(graph, start, strategy);

    loop {
        let status = astar.step(strategy);

        /*if !matches!(status, AstarStatus::Running) {
            return status;
        }*/

        match status {
            AstarStatus::Running => (),
            AstarStatus::Finished(cost, path, band) => return Ok((cost, path, band)),
            AstarStatus::Error(err) => return Err(err),
        }
    }
}
