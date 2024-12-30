// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
};

use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;
use serde::{Deserialize, Serialize};

pub trait MakeRef<'a, R: 'a, C> {
    fn ref_(&self, context: &'a C) -> R;
}

#[enum_dispatch]
pub trait GetPetgraphIndex {
    fn petgraph_index(&self) -> NodeIndex<usize>;
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
    pub fn new(index: NodeIndex<usize>) -> Self {
        Self {
            node_index: index,
            marker: PhantomData,
        }
    }
}

impl<W> core::clone::Clone for GenericIndex<W> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            node_index: self.node_index,
            marker: PhantomData,
        }
    }
}

impl<W> core::fmt::Debug for GenericIndex<W> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("GenericIndex")
            .field(&self.node_index.index())
            .finish()
    }
}

impl<W> PartialEq for GenericIndex<W> {
    #[inline]
    fn eq(&self, oth: &Self) -> bool {
        self.node_index == oth.node_index
    }
}

impl<W> core::marker::Copy for GenericIndex<W> {}
impl<W> Eq for GenericIndex<W> {}

impl<W> Hash for GenericIndex<W> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.node_index.hash(state)
    }
}

impl<W> GetPetgraphIndex for GenericIndex<W> {
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
