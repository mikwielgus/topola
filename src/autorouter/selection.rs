use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{
    autorouter::board::Board,
    drawing::{
        graph::{GetLayer, MakePrimitive, PrimitiveIndex},
        rules::RulesTrait,
    },
    graph::GenericIndex,
    layout::NodeIndex,
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Selector {
    pin: String,
    layer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selection {
    selectors: HashSet<Selector>,
}

impl Selection {
    pub fn new() -> Selection {
        Self {
            selectors: HashSet::new(),
        }
    }

    pub fn toggle_at_node(&mut self, board: &Board<impl RulesTrait>, node: NodeIndex) {
        let Some(selector) = self.node_selector(board, node) else {
            return;
        };

        if self.contains_node(board, node) {
            self.deselect(board, &selector);
        } else {
            self.select(board, selector);
        }
    }

    fn select(&mut self, board: &Board<impl RulesTrait>, selector: Selector) {
        self.selectors.insert(selector);
    }

    fn deselect(&mut self, board: &Board<impl RulesTrait>, selector: &Selector) {
        self.selectors.remove(selector);
    }

    pub fn contains_node(&self, board: &Board<impl RulesTrait>, node: NodeIndex) -> bool {
        let Some(selector) = self.node_selector(board, node) else {
            return false;
        };

        self.selectors.contains(&selector)
    }

    fn node_selector(&self, board: &Board<impl RulesTrait>, node: NodeIndex) -> Option<Selector> {
        let layer = match node {
            NodeIndex::Primitive(primitive) => {
                primitive.primitive(board.layout().drawing()).layer()
            }
            NodeIndex::Compound(compound) => board.layout().zone(compound).layer(),
        };

        if let (Some(pinname), Some(layername)) = (board.node_pinname(node), board.layername(layer))
        {
            Some(Selector {
                pin: pinname.to_string(),
                layer: layername.to_string(),
            })
        } else {
            None
        }
    }
}
