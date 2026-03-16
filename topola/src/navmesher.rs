// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use dearcut::RecordingTriangulator;
use derive_getters::Getters;

use crate::{
    Board, math,
    primitives::{Joint, JointId, Polygon, PolygonId, PrimitiveId, Segment, SegmentId, Via, ViaId},
};

#[derive(Clone, Debug, Getters)]
pub struct LayerNavmesher {
    boundary: Vec<[i64; 2]>,
    navmeshes: Vec<RecordingTriangulator<i64>>,
    navpolygon_primitive_ids: Vec<PrimitiveId>,
    inflation_factors: Vec<f64>,
}

impl LayerNavmesher {
    pub fn new(boundary: impl IntoIterator<Item = [i64; 2]>) -> Self {
        Self {
            boundary: boundary.into_iter().collect(),
            navmeshes: vec![RecordingTriangulator::new()],
            navpolygon_primitive_ids: vec![],
            inflation_factors: vec![0.0],
        }
    }

    pub fn insert_navpolygon(
        &mut self,
        primitive_id: PrimitiveId,
        polygon: impl IntoIterator<Item = [i64; 2]>,
    ) {
        let polygon: Vec<[i64; 2]> = polygon.into_iter().collect();

        for i in 0..self.navmeshes.len() {
            self.navmeshes[i].insert_polygon_and_rebuild(
                Self::inflate_polygon(polygon.clone(), self.inflation_factors[i]),
                self.boundary.clone(),
            );
        }

        self.navpolygon_primitive_ids.push(primitive_id);
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
    pub fn new(boundary: impl IntoIterator<Item = [i64; 2]>, layer_count: usize) -> Self {
        let boundary: Vec<[i64; 2]> = boundary.into_iter().collect();

        Self {
            layers: std::iter::repeat_with(|| LayerNavmesher::new(boundary.clone()))
                .take(layer_count)
                .collect(),
        }
    }

    pub fn insert_navpolygon(
        &mut self,
        layer: usize,
        primitive_id: PrimitiveId,
        polygon: impl IntoIterator<Item = [i64; 2]>,
    ) {
        self.layers[layer].insert_navpolygon(primitive_id, polygon);
    }
}

#[derive(Clone, Debug, Getters)]
pub struct NavmesherBoard {
    navmesher: Navmesher,
    board: Board,
}

impl NavmesherBoard {
    pub fn with_board(board: Board) -> Self {
        let mut navmesher = Navmesher::new(
            board.layout().boundary().clone(),
            *board.layout().layer_count(),
        );

        for (i, joint) in board.layout().joints().collection() {
            Self::insert_joint_in_navmesher(&mut navmesher, JointId::new(i), *joint);
        }

        for (i, segment) in board.layout().segments().collection() {
            Self::insert_segment_in_navmesher(&mut navmesher, &board, SegmentId::new(i), *segment);
        }

        // TODO: vias.

        for (i, polygon) in board.layout().polygons().collection() {
            Self::insert_polygon_in_navmesher(&mut navmesher, PolygonId::new(i), polygon.clone());
        }

        Self { navmesher, board }
    }

    pub fn insert_joint(&mut self, joint: Joint) -> JointId {
        let joint_id = self.board.add_joint(joint);
        Self::insert_joint_in_navmesher(&mut self.navmesher, joint_id, joint);

        joint_id
    }

    fn insert_joint_in_navmesher(navmesher: &mut Navmesher, joint_id: JointId, joint: Joint) {
        navmesher.insert_navpolygon(
            joint.layer,
            PrimitiveId::Joint(joint_id),
            Self::joint_circumscribed_octagon(joint),
        );
    }

    fn joint_circumscribed_octagon(joint: Joint) -> [[i64; 2]; 8] {
        let cx = joint.position.x;
        let cy = joint.position.y;
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

        navmesher.insert_navpolygon(
            segment.layer,
            PrimitiveId::Segment(segment_id),
            math::inflated_segment(
                endpoints[0].x,
                endpoints[0].y,
                endpoints[1].x,
                endpoints[1].y,
                segment.half_width,
            )
            .into_iter()
            .map(Into::into),
        )
    }

    pub fn insert_via(&mut self, via: Via) -> ViaId {
        // TODO: Insert into navmesh.
        self.board.add_via(via)
    }

    pub fn insert_polygon(&mut self, polygon: Polygon) -> PolygonId {
        let polygon_id = self.board.add_polygon(polygon.clone());
        Self::insert_polygon_in_navmesher(&mut self.navmesher, polygon_id, polygon);

        polygon_id
    }

    fn insert_polygon_in_navmesher(
        navmesher: &mut Navmesher,
        polygon_id: PolygonId,
        polygon: Polygon,
    ) {
        navmesher.insert_navpolygon(
            polygon.layer,
            PrimitiveId::Polygon(polygon_id),
            polygon.vertices.into_iter().map(Into::into),
        );
    }
}
