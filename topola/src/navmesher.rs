// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use dearcut::RecordingTriangulator;
use derive_getters::Getters;

use crate::{
    Board, Vector2, math,
    primitives::{Joint, JointId, Polygon, PolygonId, Segment, SegmentId, Via, ViaId},
};

#[derive(Clone, Debug, Getters)]
pub struct LayerNavmesher {
    boundary: Vec<Vector2<i64>>,
    navmeshes: Vec<RecordingTriangulator<i64>>,
    inflation_factors: Vec<f64>,
}

impl LayerNavmesher {
    pub fn new(boundary: impl IntoIterator<Item = Vector2<i64>>) -> Self {
        Self {
            boundary: boundary.into_iter().collect(),
            navmeshes: vec![RecordingTriangulator::new()],
            inflation_factors: vec![0.0],
        }
    }

    pub fn insert_polygon(&mut self, polygon: impl IntoIterator<Item = Vector2<i64>>) {
        let polygon: Vec<Vector2<i64>> = polygon.into_iter().collect();

        for i in 0..self.navmeshes.len() {
            self.navmeshes[i].insert_polygon_and_rebuild(
                Self::inflate_polygon(polygon.clone(), self.inflation_factors[i])
                    .into_iter()
                    .map(Into::into),
                self.boundary.iter().cloned().map(Into::into),
            );
        }
    }

    fn inflate_polygon(
        polygon: Vec<Vector2<i64>>,
        inflation_factor: f64,
    ) -> impl IntoIterator<Item = Vector2<i64>> {
        // Centroid.
        let cx = polygon.iter().map(|p| p.x as f64).sum::<f64>() / polygon.len() as f64;
        let cy = polygon.iter().map(|p| p.y as f64).sum::<f64>() / polygon.len() as f64;

        polygon.into_iter().map(move |p| {
            let px = p.x as f64;
            let py = p.y as f64;
            // Delta.
            let dx = px - cx;
            let dy = py - cy;
            let d = (dx * dx + dy * dy).sqrt();

            // Normalize delta.
            let nx = dx / d;
            let ny = dy / d;

            // Shift away from centroid.
            let fx = px + nx * inflation_factor;
            let fy = py + ny * inflation_factor;

            // Round away from centroid.
            let rx = if fx >= cx { fx.ceil() } else { fx.floor() };
            let ry = if fy >= cy { fy.ceil() } else { fy.floor() };

            Vector2::new(rx as i64, ry as i64)
        })
    }
}

#[derive(Clone, Debug, Getters)]
pub struct Navmesher {
    layers: Vec<LayerNavmesher>,
}

impl Navmesher {
    pub fn new(boundary: impl IntoIterator<Item = Vector2<i64>>, layer_count: usize) -> Self {
        let boundary: Vec<Vector2<i64>> = boundary.into_iter().collect();

        Self {
            layers: std::iter::repeat_with(|| LayerNavmesher::new(boundary.clone()))
                .take(layer_count)
                .collect(),
        }
    }

    pub fn insert_polygon(
        &mut self,
        layer: usize,
        polygon: impl IntoIterator<Item = Vector2<i64>>,
    ) {
        self.layers[layer].insert_polygon(polygon);
    }
}

#[derive(Clone, Debug, Getters)]
pub struct NavmesherBoard {
    navmesher: Navmesher,
    board: Board,
}

impl NavmesherBoard {
    pub fn with_board(board: Board) -> Self {
        let mut this = Self {
            navmesher: Navmesher::new(
                board
                    .layout()
                    .boundary()
                    .iter()
                    .map(|p| Vector2::new(p[0], p[1])),
                *board.layout().layer_count(),
            ),
            board,
        };

        for (_, joint) in this.board.layout().joints().collection() {
            this.navmesher
                .insert_polygon(joint.layer, Self::joint_bounding_octagon(*joint));
        }

        for (i, segment) in this.board.layout().segments().collection() {
            this.navmesher.insert_polygon(
                segment.layer,
                this.segment_bounding_rectangle(SegmentId::new(i), *segment),
            );
        }

        for (_, polygon) in this.board.layout().polygons().collection() {
            this.navmesher
                .insert_polygon(polygon.layer, polygon.vertices.clone());
        }

        this
    }

    pub fn insert_joint(&mut self, joint: Joint) -> JointId {
        self.navmesher
            .insert_polygon(joint.layer, Self::joint_bounding_octagon(joint));
        self.board.add_joint(joint)
    }

    fn joint_bounding_octagon(joint: Joint) -> [Vector2<i64>; 8] {
        let cx = joint.position.x;
        let cy = joint.position.y;
        let r = joint.radius as i64;

        [
            Vector2::new(cx + r, cy + r / 2),
            Vector2::new(cx + r / 2, cy + r),
            Vector2::new(cx - r / 2, cy + r),
            Vector2::new(cx - r, cy + r / 2),
            Vector2::new(cx - r, cy - r / 2),
            Vector2::new(cx - r / 2, cy - r),
            Vector2::new(cx + r / 2, cy - r),
            Vector2::new(cx + r, cy - r / 2),
        ]
    }

    pub fn insert_segment(&mut self, segment: Segment) -> SegmentId {
        let segment_id = self.board.add_segment(segment);
        self.navmesher.insert_polygon(
            segment.layer,
            self.segment_bounding_rectangle(segment_id, segment),
        );

        segment_id
    }

    fn segment_bounding_rectangle(
        &self,
        segment_id: SegmentId,
        segment: Segment,
    ) -> [Vector2<i64>; 4] {
        let endpoints = self.board.layout().segment_endpoints(segment_id);
        math::inflated_segment(
            endpoints[0].x,
            endpoints[0].y,
            endpoints[1].x,
            endpoints[1].y,
            segment.half_width,
        )
    }

    pub fn insert_via(&mut self, via: Via) -> ViaId {
        // TODO: Insert into navmesh.
        self.board.add_via(via)
    }

    pub fn insert_polygon(&mut self, polygon: Polygon) -> PolygonId {
        self.navmesher
            .insert_polygon(polygon.layer, polygon.vertices.clone());
        self.board.add_polygon(polygon)
    }
}
