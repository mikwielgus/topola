// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use dearcut::RecordingTriangulator;
use derive_getters::Getters;

use crate::{
    Board,
    layout::{Joint, JointId, Polygon, PolygonId, Segment, SegmentId, Via, ViaId},
};

#[derive(Clone, Debug, Getters)]
pub struct LayerNavmesher {
    navmeshes: Vec<RecordingTriangulator<i64>>,
    inflation_factors: Vec<f64>,
}

impl LayerNavmesher {
    pub fn new() -> Self {
        Self {
            navmeshes: vec![RecordingTriangulator::new()],
            inflation_factors: vec![0.0],
        }
    }

    pub fn insert_polygon(&mut self, polygon: impl IntoIterator<Item = [i64; 2]>) {
        let polygon: Vec<[i64; 2]> = polygon.into_iter().collect();

        for i in 0..self.navmeshes.len() {
            self.navmeshes[i].insert_polygon_and_rebuild(Self::inflate_polygon(
                polygon.clone(),
                self.inflation_factors[i],
            ));
        }
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

#[derive(Clone, Debug, Getters)]
pub struct Navmesher {
    layers: Vec<LayerNavmesher>,
}

impl Navmesher {
    pub fn new(layer_count: usize) -> Self {
        Self {
            layers: std::iter::repeat_with(LayerNavmesher::new)
                .take(layer_count)
                .collect(),
        }
    }

    pub fn insert_polygon(&mut self, layer: usize, polygon: impl IntoIterator<Item = [i64; 2]>) {
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
        let mut navmesher = Navmesher::new(*board.layout().layer_count());

        for (_, joint) in board.layout().joints().collection() {
            Self::insert_joint_in_navmesher(&mut navmesher, *joint);
        }

        for (i, segment) in board.layout().segments().collection() {
            Self::insert_segment_in_navmesher(&mut navmesher, &board, SegmentId::new(i), *segment);
        }

        for (_, polygon) in board.layout().polygons().collection() {
            Self::insert_polygon_in_navmesher(&mut navmesher, polygon.clone());
        }

        Self { navmesher, board }
    }

    pub fn insert_joint(&mut self, joint: Joint) -> JointId {
        Self::insert_joint_in_navmesher(&mut self.navmesher, joint);
        self.board.add_joint(joint)
    }

    fn insert_joint_in_navmesher(navmesher: &mut Navmesher, joint: Joint) {
        navmesher.insert_polygon(joint.layer, Self::joint_circumscribed_octagon(joint));
    }

    fn joint_circumscribed_octagon(joint: Joint) -> [[i64; 2]; 8] {
        let cx = joint.position[0];
        let cy = joint.position[1];
        let r = joint.radius as i64;

        [
            [cx + r, cy + r / 2],
            [cx + r / 2, cy + r],
            [cx - r / 2, cy + r],
            [cx - r, cy + r / 2],
            [cx - r, cy - r / 2],
            [cx - r / 2, cy - r],
            [cx + r / 2, cy - r],
            [cx + r, cy - r / 2],
        ]
    }

    pub fn insert_segment(&mut self, segment: Segment) -> SegmentId {
        let segment_id = self.board.add_segment(segment);
        Self::insert_segment_in_navmesher(&mut self.navmesher, &self.board, segment_id, segment);

        segment_id
    }

    fn insert_segment_in_navmesher(
        navmesher: &mut Navmesher,
        board: &Board,
        segment_id: SegmentId,
        segment: Segment,
    ) {
        let endpoints = board.layout().segment_endpoints(segment_id);

        navmesher.insert_polygon(
            segment.layer,
            Self::inflated_segment(
                endpoints[0][0],
                endpoints[0][1],
                endpoints[1][0],
                endpoints[1][1],
                segment.half_width,
            ),
        )
    }

    fn inflated_segment(x1: i64, y1: i64, x2: i64, y2: i64, half_width: u64) -> [[i64; 2]; 4] {
        let dx = x2 - x1;
        let dy = y2 - y1;

        let approx_len = std::cmp::max(dx.abs(), dy.abs()) + std::cmp::min(dx.abs(), dy.abs()) / 2;

        // Perpendicular vector scaled to half-width.
        let px = -dy * (half_width as i64) / approx_len;
        let py = dx * (half_width as i64) / approx_len;

        [
            [x1 + px, y1 + py],
            [x2 + px, y2 + py],
            [x2 - px, y2 - py],
            [x1 - px, y1 - py],
        ]
    }

    pub fn insert_via(&mut self, via: Via) -> ViaId {
        // TODO: Insert into navmesh.
        self.board.add_via(via)
    }

    pub fn insert_polygon(&mut self, polygon: Polygon) -> PolygonId {
        Self::insert_polygon_in_navmesher(&mut self.navmesher, polygon.clone());
        self.board.add_polygon(polygon)
    }

    fn insert_polygon_in_navmesher(navmesher: &mut Navmesher, polygon: Polygon) {
        navmesher.insert_polygon(polygon.layer, polygon.vertices);
    }
}
