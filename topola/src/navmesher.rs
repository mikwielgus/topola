// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use dearcut::RecordingTriangulator;
use derive_getters::Getters;

use crate::{
    Board,
    layout::{Arc, ArcId, Joint, JointId, Polygon, PolygonId, Segment, SegmentId, Via, ViaId},
};

#[derive(Clone, Debug, Getters)]
pub struct LayerNavmesher {
    navmeshes: Vec<RecordingTriangulator<i64>>,
    inflation_factors: Vec<f64>,
}

impl LayerNavmesher {
    pub fn new() -> Self {
        Self {
            navmeshes: Vec::new(),
            inflation_factors: Vec::new(),
        }
    }

    pub fn insert_polygon(&mut self, polygon: impl IntoIterator<Item = [i64; 2]>) {
        let polygon: Vec<[i64; 2]> = polygon.into_iter().collect();

        for i in 0..self.navmeshes.len() {
            self.navmeshes[i].insert_polygon(Self::inflate_polygon(
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

        // 1.082392... = 1 / cos(π/8)
        // 0.414213... = tan(π/8)

        // Approximate multipliers as fractions.
        let r1 = (r * 277 + 128) / 256; // round(r * 1.0823922)
        let r2 = (r * 106 + 128) / 256; // round(r * 0.41421356)

        [
            [cx + r1, cy],      // right
            [cx + r2, cy + r2], // top-right
            [cx, cy + r1],      // top
            [cx - r2, cy + r2], // top-left
            [cx - r1, cy],      // left
            [cx - r2, cy - r2], // bottom-left
            [cx, cy - r1],      // bottom
            [cx + r2, cy - r2], // bottom-right
        ]
    }

    pub fn insert_segment(&mut self, segment: Segment) -> SegmentId {
        // TODO: Insert into navmesh.
        self.board.add_segment(segment)
    }

    pub fn insert_via(&mut self, via: Via) -> ViaId {
        // TODO: Insert into navmesh.
        self.board.add_via(via)
    }

    pub fn insert_polygon(&mut self, polygon: Polygon) -> PolygonId {
        // TODO: Insert into navmesh.
        self.board.add_polygon(polygon)
    }
}
