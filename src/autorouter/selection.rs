use core::fmt;
use std::collections::HashSet;

use crate::{
    autorouter::board::{Board, NodeIndex},
    drawing::{graph::PrimitiveIndex, rules::RulesTrait},
    graph::GenericIndex,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Selection {
    pins: HashSet<String>,
}

impl Selection {
    pub fn new() -> Selection {
        Self {
            pins: HashSet::new(),
        }
    }

    pub fn toggle_at_node(&mut self, board: &Board<impl RulesTrait>, node: NodeIndex) {
        let maybe_pin = board.node_pin(node);

        if let Some(ref pin) = maybe_pin {
            if self.contains_node(board, node) {
                self.remove_pin(board, pin);
            } else {
                self.add_pin(board, pin);
            }
        }
    }

    fn add_pin(&mut self, board: &Board<impl RulesTrait>, pin: &String) {
        self.pins.insert(pin.clone());
    }

    fn remove_pin(&mut self, board: &Board<impl RulesTrait>, pin: &String) {
        self.pins.remove(pin);
    }

    pub fn contains_node(&self, board: &Board<impl RulesTrait>, node: NodeIndex) -> bool {
        if let Some(pin) = board.node_pin(node) {
            self.pins.contains(pin)
        } else {
            false
        }
    }
}
