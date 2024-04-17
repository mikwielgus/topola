use std::collections::HashSet;

use geo::Point;
use rstar::AABB;

use crate::{
    drawing::{
        graph::{GetLayer, MakePrimitive, PrimitiveIndex},
        primitive::MakeShape,
        rules::RulesTrait,
    },
    geometry::shape::ShapeTrait,
    graph::GenericIndex,
    layout::{zone::ZoneWeight, Layout, NodeIndex},
};

pub struct Overlay {
    selection: HashSet<NodeIndex>,
    active_layer: u64,
}

impl Overlay {
    pub fn new() -> Self {
        Self {
            selection: HashSet::new(),
            active_layer: 0,
        }
    }

    pub fn click<R: RulesTrait>(&mut self, layout: &Layout<R>, at: Point) {
        let geoms: Vec<_> = layout
            .drawing()
            .rtree()
            .locate_in_envelope_intersecting(&AABB::<[f64; 3]>::from_corners(
                [at.x(), at.y(), -f64::INFINITY],
                [at.x(), at.y(), f64::INFINITY],
            ))
            .collect();

        if let Some(geom) = geoms.iter().find(|&&geom| match geom.data {
            NodeIndex::Primitive(primitive) => {
                primitive.primitive(layout.drawing()).layer() == self.active_layer
            }
            NodeIndex::Compound(compound) => false,
        }) {
            if self.toggle_selection_if_contains_point(layout, geom.data, at) {
                return;
            }
        }

        for geom in geoms {
            if self.toggle_selection_if_contains_point(layout, geom.data, at) {
                return;
            }
        }
    }

    fn toggle_selection_if_contains_point<R: RulesTrait>(
        &mut self,
        layout: &Layout<R>,
        node: NodeIndex,
        p: Point,
    ) -> bool {
        match node {
            NodeIndex::Primitive(primitive) => {
                if primitive
                    .primitive(layout.drawing())
                    .shape()
                    .contains_point(p)
                {
                    self.toggle_selection(node);
                    return true;
                }
            }
            NodeIndex::Compound(compound) => (), // TODO.
        }

        false
    }

    fn toggle_selection(&mut self, node: NodeIndex) {
        if !self.selection.insert(node) {
            self.selection.remove(&node);
        }
    }

    pub fn selection(&self) -> &HashSet<NodeIndex> {
        &self.selection
    }
}
