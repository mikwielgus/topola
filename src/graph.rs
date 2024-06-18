use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
};

use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;

// Due to apparent limitations of enum_dispatch we're forced to import some types backwards.

#[enum_dispatch]
pub trait GetPetgraphIndex {
    fn petgraph_index(&self) -> NodeIndex<usize>;
}

#[derive(Debug, Clone, Copy)]
pub struct GenericIndex<W> {
    node_index: NodeIndex<usize>,
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

impl<W> Hash for GenericIndex<W> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.node_index.hash(state)
    }
}

impl<W> PartialEq for GenericIndex<W> {
    fn eq(&self, other: &Self) -> bool {
        self.node_index == other.node_index
    }
}

impl<W> Eq for GenericIndex<W> {}

impl<W> GetPetgraphIndex for GenericIndex<W> {
    fn petgraph_index(&self) -> NodeIndex<usize> {
        self.node_index
    }
}
