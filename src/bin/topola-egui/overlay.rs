use std::collections::HashSet;

use geo::Point;
use rstar::AABB;
use spade::InsertionError;

use topola::{
    autorouter::{ratsnest::Ratsnest, selection::Selection},
    board::{mesadata::MesadataTrait, Board},
    drawing::{
        graph::{GetLayer, MakePrimitive},
        primitive::MakePrimitiveShape,
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
    pub fn new(board: &Board<impl MesadataTrait>) -> Result<Self, InsertionError> {
        Ok(Self {
            ratsnest: Ratsnest::new(board.layout())?,
            selection: Selection::new(),
            active_layer: 0,
        })
    }

    pub fn click(&mut self, board: &Board<impl MesadataTrait>, at: Point) {
        let geoms: Vec<_> = board
            .layout()
            .drawing()
            .rtree()
            .locate_in_envelope_intersecting(&AABB::<[f64; 3]>::from_corners(
                [at.x(), at.y(), -f64::INFINITY],
                [at.x(), at.y(), f64::INFINITY],
            ))
            .collect();

        if let Some(geom) = geoms.iter().find(|&&geom| match geom.data {
            NodeIndex::Primitive(primitive) => {
                primitive.primitive(board.layout().drawing()).layer() == self.active_layer
            }
            NodeIndex::Compound(compound) => false,
        }) {
            if self.toggle_selection_if_contains_point(board, geom.data, at) {
                return;
            }
        }

        for geom in geoms {
            if self.toggle_selection_if_contains_point(board, geom.data, at) {
                return;
            }
        }
    }

    fn toggle_selection_if_contains_point(
        &mut self,
        board: &Board<impl MesadataTrait>,
        node: NodeIndex,
        p: Point,
    ) -> bool {
        let shape: Shape = match node {
            NodeIndex::Primitive(primitive) => {
                primitive.primitive(board.layout().drawing()).shape().into()
            }
            NodeIndex::Compound(compound) => board.layout().zone(compound).shape().into(),
        };

        if shape.contains_point(p) {
            self.selection.toggle_at_node(board, node);
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
