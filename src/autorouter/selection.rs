use std::collections::HashSet;

use crate::{
    drawing::{graph::PrimitiveIndex, rules::RulesTrait},
    graph::GenericIndex,
    layout::{zone::ZoneWeight, Layout, NodeIndex},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Selection {
    pins: HashSet<String>,
    #[serde(skip)]
    primitives: HashSet<PrimitiveIndex>,
    #[serde(skip)]
    zones: HashSet<GenericIndex<ZoneWeight>>,
}

impl Selection {
    pub fn new() -> Selection {
        Self {
            pins: HashSet::new(),
            primitives: HashSet::new(),
            zones: HashSet::new(),
        }
    }

    pub fn toggle_at_node(&mut self, layout: &Layout<impl RulesTrait>, node: NodeIndex) {
        let maybe_pin = layout.node_pin(node);

        if let Some(ref pin) = maybe_pin {
            if self.contains(node) {
                self.remove_pin(layout, pin);
            } else {
                self.add_pin(layout, pin);
            }
        }
    }

    fn add_pin(&mut self, layout: &Layout<impl RulesTrait>, pin: &String) {
        for primitive in layout.drawing().primitive_nodes().filter(|primitive| {
            layout
                .node_pin(NodeIndex::Primitive(*primitive))
                .is_some_and(|p| p == pin)
        }) {
            self.primitives.insert(primitive);
        }

        for zone in layout.zone_nodes().filter(|zone| {
            layout
                .node_pin(NodeIndex::Compound(*zone))
                .is_some_and(|p| p == pin)
        }) {
            self.zones.insert(zone);
        }

        self.pins.insert(pin.clone());
    }

    fn remove_pin(&mut self, layout: &Layout<impl RulesTrait>, pin: &String) {
        for primitive in layout.drawing().primitive_nodes().filter(|primitive| {
            layout
                .node_pin(NodeIndex::Primitive(*primitive))
                .is_some_and(|p| p == pin)
        }) {
            self.primitives.remove(&primitive);
        }

        for zone in layout.zone_nodes().filter(|zone| {
            layout
                .node_pin(NodeIndex::Compound(*zone))
                .is_some_and(|p| p == pin)
        }) {
            self.zones.remove(&zone);
        }

        self.pins.remove(pin);
    }

    pub fn contains(&self, node: NodeIndex) -> bool {
        match node {
            NodeIndex::Primitive(primitive) => self.primitives.contains(&primitive),
            NodeIndex::Compound(zone) => self.zones.contains(&zone),
        }
    }
}
