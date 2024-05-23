use core::fmt;
use std::collections::HashSet;

use crate::{
    drawing::{graph::PrimitiveIndex, rules::RulesTrait},
    graph::GenericIndex,
    layout::{zone::ZoneWeight, Layout, NodeIndex},
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

    pub fn toggle_at_node(&mut self, layout: &Layout<impl RulesTrait>, node: NodeIndex) {
        let maybe_pin = layout.node_pin(node);

        if let Some(ref pin) = maybe_pin {
            if self.contains_node(layout, node) {
                self.remove_pin(layout, pin);
            } else {
                self.add_pin(layout, pin);
            }
        }
    }

    fn add_pin(&mut self, layout: &Layout<impl RulesTrait>, pin: &String) {
        self.pins.insert(pin.clone());
    }

    fn remove_pin(&mut self, layout: &Layout<impl RulesTrait>, pin: &String) {
        self.pins.remove(pin);
    }

    pub fn contains_node(&self, layout: &Layout<impl RulesTrait>, node: NodeIndex) -> bool {
        if let Some(pin) = layout.node_pin(node) {
            self.pins.contains(pin)
        } else {
            false
        }
    }
}
