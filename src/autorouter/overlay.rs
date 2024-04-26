use std::collections::HashSet;

use geo::Point;
use rstar::AABB;
use spade::InsertionError;

use crate::{
    autorouter::ratsnest::Ratsnest,
    drawing::{
        graph::{GetLayer, MakePrimitive},
        primitive::MakePrimitiveShape,
        rules::RulesTrait,
    },
    geometry::{
        compound::CompoundManagerTrait,
        shape::{Shape, ShapeTrait},
    },
    layout::{zone::MakePolyShape, Layout, NodeIndex},
};

pub struct Overlay {
    ratsnest: Ratsnest,
    selection: HashSet<NodeIndex>,
    active_layer: u64,
}

impl Overlay {
    pub fn new(layout: &Layout<impl RulesTrait>) -> Result<Self, InsertionError> {
        Ok(Self {
            ratsnest: Ratsnest::new(layout)?,
            selection: HashSet::new(),
            active_layer: 0,
        })
    }

    pub fn click(&mut self, layout: &Layout<impl RulesTrait>, at: Point) {
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

    fn toggle_selection_if_contains_point(
        &mut self,
        layout: &Layout<impl RulesTrait>,
        node: NodeIndex,
        p: Point,
    ) -> bool {
        let shape: Shape = match node {
            NodeIndex::Primitive(primitive) => primitive.primitive(layout.drawing()).shape().into(),
            NodeIndex::Compound(compound) => layout
                .compound_weight(compound)
                .shape(layout.drawing(), compound)
                .into(),
        };

        if shape.contains_point(p) {
            self.toggle_selection(node);
            return true;
        }
        false
    }

    pub fn toggle_selection(&mut self, node: NodeIndex) {
        if !self.selection.insert(node) {
            self.selection.remove(&node);
        }
    }

    pub fn ratsnest(&self) -> &Ratsnest {
        &self.ratsnest
    }

    pub fn selection(&self) -> &HashSet<NodeIndex> {
        &self.selection
    }
}
