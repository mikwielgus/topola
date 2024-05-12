use std::collections::HashSet;

use crate::layout::NodeIndex;

#[derive(Debug, Clone)]
pub struct Selection {
    set: HashSet<NodeIndex>,
}

impl Selection {
    pub fn new() -> Selection {
        Self {
            set: HashSet::new(),
        }
    }

    pub fn toggle_at_node(&mut self, node: NodeIndex) {
        if !self.set.insert(node) {
            self.set.remove(&node);
        }
    }

    pub fn contains(&self, node: &NodeIndex) -> bool {
        self.set.contains(node)
    }
}
