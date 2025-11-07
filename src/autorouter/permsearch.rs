// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{cmp::Ordering, collections::BinaryHeap, iter::Skip, iter::Take};

use itertools::{Itertools, Permutations};

#[derive(Clone, Debug)]
struct PermsearchNode<T> {
    curr_permutation: Vec<T>,
    permutations: Skip<Permutations<Take<std::vec::IntoIter<T>>>>,
    length: usize,
}

impl<T: Eq> PartialEq for PermsearchNode<T> {
    fn eq(&self, other: &Self) -> bool {
        self.curr_permutation == other.curr_permutation
    }
}

impl<T: Eq> Eq for PermsearchNode<T> {}

impl<T: Eq> Ord for PermsearchNode<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.length.cmp(&other.length)
    }
}

impl<T: Eq> PartialOrd for PermsearchNode<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.length.partial_cmp(&other.length)
    }
}

impl<T: Clone> PermsearchNode<T> {
    fn permute(mut self) -> Option<Self> {
        for (i, element) in self.permutations.next()?.iter().enumerate() {
            self.curr_permutation[i] = element.clone();
        }

        Some(self)
    }

    fn resize(self, length: usize) -> Option<Self> {
        if length == self.length {
            return None;
        }

        // TODO: Get rid of `self.curr_permutation` clone somehow?

        Some(Self {
            curr_permutation: self.curr_permutation.clone(),
            permutations: self
                .curr_permutation
                .into_iter()
                .take(length)
                .permutations(length)
                .skip(1),
            length,
        })
    }
}

pub struct Permsearch<T> {
    curr_node: PermsearchNode<T>,
    frontier: BinaryHeap<PermsearchNode<T>>,
}

impl<T: Eq + Clone> Permsearch<T> {
    pub fn new(original: Vec<T>) -> Self {
        let len = original.len();

        // TODO: Get rid of `original` clone somehow?

        Self {
            curr_node: PermsearchNode {
                curr_permutation: original.clone(),
                permutations: original.into_iter().take(len).permutations(0).skip(0),
                length: 0,
            },
            frontier: BinaryHeap::new(),
        }
    }

    pub fn step(&mut self, len: usize) -> Option<&[T]> {
        // TODO: Get rid of `self.curr_node` clones somehow?

        if let Some(resized_curr_node) = self.curr_node.clone().resize(len) {
            if let Some(permuted_resized_curr_node) = resized_curr_node.permute() {
                self.frontier.push(permuted_resized_curr_node);
            }
        }

        if let Some(permuted_curr_node) = self.curr_node.clone().permute() {
            self.frontier.push(permuted_curr_node);
        }

        self.curr_node = self.frontier.pop()?;
        Some(&self.curr_node.curr_permutation)
    }

    pub fn curr_permutation(&self) -> &[T] {
        &self.curr_node.curr_permutation
    }
}
