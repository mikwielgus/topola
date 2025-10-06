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
    pub fn polys_enclosing_point_on_layers(
        &self,
        point: Point,
        layer: usize,
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

                if !self.drawing.compound_weight(compound).is_in_layer(layer)
                    || !self.node_shape(node).contains_point(point)
                {
                    return None;
                }

                Some(GenericIndex::<PolyWeight>::new(compound.index()))
            })
    }
}
