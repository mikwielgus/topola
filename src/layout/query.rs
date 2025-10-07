// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::Point;
use rstar::AABB;
use specctra_core::rules::AccessRules;

use crate::{
    drawing::graph::IsInLayer,
    geometry::{shape::AccessShape, GenericNode},
    graph::{GenericIndex, GetIndex},
    layout::{poly::PolyWeight, CompoundWeight, Layout},
};

impl<R: AccessRules> Layout<R> {
    pub fn polys_enclosing_point(
        &self,
        point: Point,
    ) -> impl Iterator<Item = GenericIndex<PolyWeight>> + '_ {
        self.drawing()
            .rtree()
            .locate_in_envelope_intersecting(&AABB::<[f64; 3]>::from_corners(
                [point.x(), point.y(), -f64::INFINITY],
                [point.x(), point.y(), f64::INFINITY],
            ))
            .filter_map(move |geom| {
                let node = geom.data;

                let GenericNode::Compound(compound) = node else {
                    return None;
                };

                let CompoundWeight::Poly(_) = self.drawing.compound_weight(compound) else {
                    return None;
                };

                if !self.node_shape(node).contains_point(point) {
                    return None;
                }

                Some(GenericIndex::<PolyWeight>::new(compound.index()))
            })
    }

    pub fn polys_enclosing_point_on_layer(
        &self,
        point: Point,
        layer: usize,
    ) -> impl Iterator<Item = GenericIndex<PolyWeight>> + '_ {
        self.polys_enclosing_point(point).filter(move |node| {
            self.drawing
                .compound_weight((*node).into())
                .is_in_layer(layer)
        })
    }
}
