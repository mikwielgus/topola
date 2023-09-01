/**
 *
 * Copied from petgraph's scored.rs and algo/astar.rs. Renamed the `is_goal: IsGoal` callback to
 * `reroute: Reroute` and made it pass a reference to `path_tracker` and return a value to be added
 * to outgoing edge costs.
 *
 * Copyright (c) 2015
 **/
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::{BinaryHeap, HashMap};

use std::hash::Hash;

use petgraph::algo::Measure;
use petgraph::visit::{EdgeRef, GraphBase, IntoEdges, Visitable};

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

pub trait AstarStrategy<G, K>
where
    G: IntoEdges + Visitable,
    K: Measure + Copy,
    G::NodeId: Eq + Hash,
{
    fn reroute(&mut self, node: G::NodeId, tracker: &PathTracker<G>) -> Option<K>;
    fn edge_cost(&mut self, edge: G::EdgeRef) -> K;
    fn estimate_cost(&mut self, node: G::NodeId) -> K;
}

pub fn astar<G, K>(
    graph: G,
    start: G::NodeId,
    strategy: &mut impl AstarStrategy<G, K>,
) -> Option<(K, Vec<G::NodeId>)>
where
    G: IntoEdges + Visitable,
    G::NodeId: Eq + Hash,
    K: Measure + Copy,
{
    let mut visit_next = BinaryHeap::new();
    let mut scores = HashMap::new(); // g-values, cost to reach the node
    let mut estimate_scores = HashMap::new(); // f-values, cost to reach + estimate cost to goal
    let mut path_tracker = PathTracker::<G>::new();

    let zero_score = K::default();
    scores.insert(start, zero_score);
    visit_next.push(MinScored(strategy.estimate_cost(start), start));

    while let Some(MinScored(estimate_score, node)) = visit_next.pop() {
        match strategy.reroute(node, &path_tracker) {
            None => {
                let path = path_tracker.reconstruct_path_to(node);
                let cost = scores[&node];
                return Some((cost, path));
            }
            Some(route_cost) => {
                // This lookup can be unwrapped without fear of panic since the node was
                // necessarily scored before adding it to `visit_next`.
                let node_score = scores[&node];

                match estimate_scores.entry(node) {
                    Occupied(mut entry) => {
                        // If the node has already been visited with an equal or lower score than
                        // now, then we do not need to re-visit it.
                        if *entry.get() <= estimate_score {
                            continue;
                        }
                        entry.insert(estimate_score);
                    }
                    Vacant(entry) => {
                        entry.insert(estimate_score);
                    }
                }

                for edge in graph.edges(node) {
                    let next = edge.target();
                    let next_score = node_score + route_cost + strategy.edge_cost(edge);

                    match scores.entry(next) {
                        Occupied(mut entry) => {
                            // No need to add neighbors that we have already reached through a
                            // shorter path than now.
                            if *entry.get() <= next_score {
                                continue;
                            }
                            entry.insert(next_score);
                        }
                        Vacant(entry) => {
                            entry.insert(next_score);
                        }
                    }

                    path_tracker.set_predecessor(next, node);
                    let next_estimate_score = next_score + strategy.estimate_cost(next);
                    visit_next.push(MinScored(next_estimate_score, next));
                }
            }
        }
    }

    None
}
