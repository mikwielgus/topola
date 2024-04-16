use std::collections::HashSet;

use geo::Point;
use rstar::AABB;

use crate::{
    drawing::{graph::PrimitiveIndex, rules::RulesTrait},
    geometry::Node,
    graph::GenericIndex,
    layout::{zone::ZoneWeight, Layout},
};

pub struct Overlay {
    selection: HashSet<Node<PrimitiveIndex, GenericIndex<ZoneWeight>>>,
}

impl Overlay {
    pub fn new() -> Self {
        Self {
            selection: HashSet::new(),
        }
    }

    pub fn click<R: RulesTrait>(&mut self, layout: &Layout<R>, at: Point) {
        for geom in layout.drawing().rtree().locate_in_envelope_intersecting(
            &AABB::<[f64; 3]>::from_corners(
                [at.x(), at.y(), -f64::INFINITY],
                [at.x(), at.y(), f64::INFINITY],
            ),
        ) {
            self.toggle_selection(geom.data);
        }
    }

    fn toggle_selection(&mut self, node: Node<PrimitiveIndex, GenericIndex<ZoneWeight>>) {
        if !self.selection.insert(node) {
            self.selection.remove(&node);
        }
    }

    pub fn selection(&self) -> &HashSet<Node<PrimitiveIndex, GenericIndex<ZoneWeight>>> {
        &self.selection
    }
}
