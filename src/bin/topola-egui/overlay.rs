use std::collections::HashSet;

use geo::Point;
use rstar::AABB;
use spade::InsertionError;

use topola::{
    autorouter::{ratsnest::Ratsnest, selection::Selection},
    board::{mesadata::AccessMesadata, Board},
    drawing::{
        graph::{GetLayer, MakePrimitive},
        primitive::MakePrimitiveShape,
    },
    geometry::{
        compound::ManageCompounds,
        shape::{AccessShape, Shape},
    },
    graph::{GenericIndex, GetPetgraphIndex},
    layout::{
        poly::{MakePolyShape, Poly, PolyWeight},
        via::ViaWeight,
        CompoundWeight, Layout, NodeIndex,
    },
};

pub struct Overlay {
    ratsnest: Ratsnest,
    selection: Selection,
    active_layer: usize,
}

impl Overlay {
    pub fn new(board: &Board<impl AccessMesadata>) -> Result<Self, InsertionError> {
        Ok(Self {
            ratsnest: Ratsnest::new(board.layout())?,
            selection: Selection::new(),
            active_layer: 0,
        })
    }

    pub fn clear_selection(&mut self) {
        self.selection = Selection::new();
    }

    pub fn click(&mut self, board: &Board<impl AccessMesadata>, at: Point) {
        let geoms: Vec<_> = board
            .layout()
            .drawing()
            .rtree()
            .locate_in_envelope_intersecting(&AABB::<[f64; 3]>::from_corners(
                [at.x(), at.y(), -f64::INFINITY],
                [at.x(), at.y(), f64::INFINITY],
            ))
            .collect();

        if let Some(geom) = geoms.iter().find(|&&geom| {
            self.contains_point(board, geom.data, at)
                && match geom.data {
                    NodeIndex::Primitive(primitive) => {
                        primitive.primitive(board.layout().drawing()).layer() == self.active_layer
                    }
                    NodeIndex::Compound(compound) => {
                        match board.layout().drawing().compound_weight(compound) {
                            CompoundWeight::Poly(weight) => {
                                board
                                    .layout()
                                    .poly(GenericIndex::<PolyWeight>::new(
                                        compound.petgraph_index(),
                                    ))
                                    .layer()
                                    == self.active_layer
                            }
                            CompoundWeight::Via(weight) => {
                                weight.from_layer >= self.active_layer
                                    && weight.to_layer <= self.active_layer
                            }
                        }
                    }
                }
        }) {
            self.selection.toggle_at_node(board, geom.data);
        }
    }

    fn contains_point(
        &self,
        board: &Board<impl AccessMesadata>,
        node: NodeIndex,
        p: Point,
    ) -> bool {
        let shape: Shape = match node {
            NodeIndex::Primitive(primitive) => {
                primitive.primitive(board.layout().drawing()).shape().into()
            }
            NodeIndex::Compound(compound) => {
                match board.layout().drawing().compound_weight(compound) {
                    CompoundWeight::Poly(weight) => board
                        .layout()
                        .poly(GenericIndex::<PolyWeight>::new(compound.petgraph_index()))
                        .shape()
                        .into(),
                    CompoundWeight::Via(weight) => board
                        .layout()
                        .via(GenericIndex::<ViaWeight>::new(compound.petgraph_index()))
                        .shape()
                        .into(),
                }
            }
        };

        shape.contains_point(p)
    }

    pub fn ratsnest(&self) -> &Ratsnest {
        &self.ratsnest
    }

    pub fn selection(&self) -> &Selection {
        &self.selection
    }
}
