// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{
    collections::{btree_map::Entry, BTreeMap, BinaryHeap},
    ops::Add,
};

use derive_getters::Getters;

use crate::scored::MinScored;

#[derive(Getters)]
pub struct Astar<N, S> {
    #[getter(skip)]
    frontier: BinaryHeap<MinScored<S, N>>,
    #[getter(skip)]
    g_scores: BTreeMap<N, S>,
    curr_node: N,
}

impl<N: Clone + Ord, S: Add<S, Output = S> + Copy + Default + PartialOrd> Astar<N, S> {
    pub fn new(start: N) -> Self {
        let mut frontier = BinaryHeap::new();
        let mut scores = BTreeMap::new();

        scores.insert(start.clone(), S::default());
        frontier.push(MinScored(S::default(), start.clone()));

        Self {
            frontier,
            g_scores: scores,
            curr_node: start,
        }
    }

    pub fn expand(&mut self, new_nodes: &[(S, S, N)]) -> Option<N> {
        for new_node in new_nodes {
            self.push(new_node.clone());
        }

        self.pop()
    }

    pub fn push(&mut self, new_node: (S, S, N)) {
        let curr_g_score = self.g_scores.get(&self.curr_node).unwrap().clone();
        let (edge_g_cost, h_heuristic, node) = new_node;

        match self.g_scores.entry(node.clone()) {
            Entry::Occupied(mut entry) => {
                let entry_score = *entry.get();

                if curr_g_score + edge_g_cost >= entry_score {
                    return;
                }

                entry.insert(curr_g_score + edge_g_cost);
            }
            Entry::Vacant(entry) => {
                entry.insert(curr_g_score + edge_g_cost);
            }
        }

        self.frontier
            .push(MinScored(curr_g_score + edge_g_cost + h_heuristic, node));
    }

    pub fn pop(&mut self) -> Option<N> {
        MinScored(_ /*f_score*/, self.curr_node) = self.frontier.pop()?;
        Some(self.curr_node.clone())
    }
}
