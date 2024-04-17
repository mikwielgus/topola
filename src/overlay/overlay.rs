use std::collections::HashSet;

use geo::Point;
use rstar::AABB;

use crate::{
    drawing::{
        graph::{GetLayer, MakePrimitive, PrimitiveIndex},
        primitive::MakeShape,
        rules::RulesTrait,
    },
    geometry::{shape::ShapeTrait, Node},
    graph::GenericIndex,
    layout::{zone::ZoneWeight, Layout},
};

pub struct Overlay {
    selection: HashSet<Node<PrimitiveIndex, GenericIndex<ZoneWeight>>>,
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
            Node::Primitive(primitive) => {
                primitive.primitive(layout.drawing()).layer() == self.active_layer
            }
            Node::Compound(compound) => false,
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
        node: Node<PrimitiveIndex, GenericIndex<ZoneWeight>>,
        p: Point,
    ) -> bool {
        match node {
            Node::Primitive(primitive) => {
                if primitive
                    .primitive(layout.drawing())
                    .shape()
                    .contains_point(p)
                {
                    self.toggle_selection(node);
                    return true;
                }
            }
            Node::Compound(compound) => (), // TODO.
        }

        false
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
