use std::collections::HashSet;

use geo::Point;
use rstar::AABB;
use spade::InsertionError;

use topola::{
    autorouter::{ratsnest::Ratsnest, selection::Selection},
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
    selection: Selection,
    active_layer: u64,
}

impl Overlay {
    pub fn new(layout: &Layout<impl RulesTrait>) -> Result<Self, InsertionError> {
        Ok(Self {
            ratsnest: Ratsnest::new(layout)?,
            selection: Selection::new(),
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
            NodeIndex::Compound(compound) => layout.zone(compound).shape().into(),
        };

        if shape.contains_point(p) {
            self.selection.toggle_at_node(layout, node);
            return true;
        }
        false
    }

    pub fn ratsnest(&self) -> &Ratsnest {
        &self.ratsnest
    }

    pub fn selection(&self) -> &Selection {
        &self.selection
    }
}
