// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{cmp::Ordering, marker::PhantomData};

use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;
use serde::{Deserialize, Serialize};

pub trait MakeRef<'a, C> {
    type Output: 'a;
    fn ref_(&self, context: &'a C) -> Self::Output;
}

#[enum_dispatch]
pub trait GetPetgraphIndex {
    fn petgraph_index(&self) -> NodeIndex<usize>;
}

impl GetPetgraphIndex for NodeIndex<usize> {
    #[inline(always)]
    fn petgraph_index(&self) -> NodeIndex<usize> {
        *self
    }
}

// unfortunately, as we don't want any restrictions on `W`,
// we have to implement many traits ourselves, instead of using derive macros.
#[derive(Deserialize, Serialize)]
#[serde(bound = "")]
#[serde(transparent)]
pub struct GenericIndex<W> {
    node_index: NodeIndex<usize>,
    #[serde(skip)]
    marker: PhantomData<W>,
}

impl<W> GenericIndex<W> {
    #[inline]
    pub fn new(index: NodeIndex<usize>) -> Self {
        Self {
            node_index: index,
            marker: PhantomData,
        }
    }
}

impl<W> core::clone::Clone for GenericIndex<W> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<W> core::marker::Copy for GenericIndex<W> {}

impl<W> core::fmt::Debug for GenericIndex<W> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.node_index.index(), f)
    }
}

impl<W> PartialEq for GenericIndex<W> {
    #[inline]
    fn eq(&self, oth: &Self) -> bool {
        self.node_index == oth.node_index
    }
}

impl<W> Eq for GenericIndex<W> {}

impl<W> PartialOrd for GenericIndex<W> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<W> Ord for GenericIndex<W> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.node_index.cmp(&other.node_index)
    }
}

impl<W> core::hash::Hash for GenericIndex<W> {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.node_index.hash(state);
    }
}

impl<W> GetPetgraphIndex for GenericIndex<W> {
    #[inline]
    fn petgraph_index(&self) -> NodeIndex<usize> {
        self.node_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializable_index() {
        assert_eq!(
            serde_json::to_string(&GenericIndex::<()>::new(NodeIndex::new(0))).unwrap(),
            "0"
        );
        assert_eq!(
            serde_json::from_str::<GenericIndex<()>>("0").unwrap(),
            GenericIndex::new(NodeIndex::new(0))
        );
    }
}
