// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use dearcut::RecordingTriangulator;
use derive_getters::Getters;

use crate::primitives::PrimitiveId;

#[derive(Clone, Debug, Getters)]
pub struct Navmesh {
    boundary: Vec<[i64; 2]>,
    triangulator: RecordingTriangulator<i64>,
    navpolygon_primitives: Vec<PrimitiveId>,
    inflation_factor: f64,
}

impl Navmesh {
    pub fn new(boundary: impl IntoIterator<Item = [i64; 2]>) -> Self {
        Self {
            boundary: boundary.into_iter().collect(),
            triangulator: RecordingTriangulator::new(),
            navpolygon_primitives: Vec::new(),
            inflation_factor: 0.0,
        }
    }

    pub fn insert_polygon(
        &mut self,
        primitive_id: PrimitiveId,
        polygon: impl IntoIterator<Item = [i64; 2]>,
    ) {
        let navpolygon_index = self.triangulator.insert_polygon_and_rebuild(
            Self::inflate_polygon(polygon, self.inflation_factor),
            self.boundary.clone(),
        );

        self.navpolygon_primitives
            .resize(navpolygon_index + 1, primitive_id);
        self.navpolygon_primitives[navpolygon_index] = primitive_id;
    }

    fn inflate_polygon(
        polygon: impl IntoIterator<Item = [i64; 2]>,
        inflation_factor: f64,
    ) -> impl IntoIterator<Item = [i64; 2]> {
        let polygon: Vec<[i64; 2]> = polygon.into_iter().collect();

        // Centroid.
        let cx = polygon.iter().map(|p| p[0] as f64).sum::<f64>() / polygon.len() as f64;
        let cy = polygon.iter().map(|p| p[1] as f64).sum::<f64>() / polygon.len() as f64;

        polygon.into_iter().map(move |[px, py]| {
            // Delta.
            let dx = px as f64 - cx;
            let dy = py as f64 - cy;
            let d = (dx * dx + dy * dy).sqrt();

            // Normalize delta.
            let nx = dx / d;
            let ny = dy / d;

            // Shift away from centroid.
            let fx = px as f64 + nx * inflation_factor;
            let fy = py as f64 + ny * inflation_factor;

            // Round away from centroid.
            let rx = if fx >= cx { fx.ceil() } else { fx.floor() };
            let ry = if fy >= cy { fy.ceil() } else { fy.floor() };

            [rx as i64, ry as i64]
        })
    }
}
