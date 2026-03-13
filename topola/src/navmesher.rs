// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_getters::Getters;

use crate::{
    Board,
    navmesh::Navmesh,
    primitives::{Joint, JointId, Polygon, PolygonId, PrimitiveId, Segment, SegmentId, Via, ViaId},
};

#[derive(Clone, Debug, Getters)]
pub struct LayerNavmesher {
    boundary: Vec<[i64; 2]>,
    navmeshes: Vec<Navmesh>,
}

impl LayerNavmesher {
    fn new(boundary: impl IntoIterator<Item = [i64; 2]>) -> Self {
        let boundary: Vec<[i64; 2]> = boundary.into_iter().collect();

        Self {
            boundary: boundary.clone(),
            navmeshes: vec![Navmesh::new(boundary)],
        }
    }

    fn insert_primitive_in_polygon(
        &mut self,
        primitive_id: PrimitiveId,
        polygon: impl IntoIterator<Item = [i64; 2]>,
    ) {
        let polygon: Vec<[i64; 2]> = polygon.into_iter().collect();

        for i in 0..self.navmeshes.len() {
            self.navmeshes[i].insert_polygon(primitive_id, polygon.clone());
        }
    }
}

#[derive(Clone, Debug, Getters)]
pub struct Navmesher {
    layers: Vec<LayerNavmesher>,
}

impl Navmesher {
    fn new(boundary: impl IntoIterator<Item = [i64; 2]>, layer_count: usize) -> Self {
        let boundary: Vec<[i64; 2]> = boundary.into_iter().collect();

        Self {
            layers: std::iter::repeat_with(|| LayerNavmesher::new(boundary.clone()))
                .take(layer_count)
                .collect(),
        }
    }

    fn insert_joint(&mut self, joint_id: JointId, joint: Joint) {
        self.layers[joint.layer]
            .insert_primitive_in_polygon(joint_id.into(), Self::joint_circumscribed_octagon(joint));
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

    fn insert_segment(&mut self, board: &Board, segment_id: SegmentId, segment: Segment) {
        let endpoints = board.layout().segment_endpoints(segment_id);

        self.layers[segment.layer].insert_primitive_in_polygon(
            segment_id.into(),
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

        let approx_len =
            std::cmp::max(dx.abs(), dy.abs()) + 3 * std::cmp::min(dx.abs(), dy.abs()) / 8;

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

    fn insert_polygon(&mut self, polygon_id: PolygonId, polygon: Polygon) {
        self.layers[polygon.layer].insert_primitive_in_polygon(polygon_id.into(), polygon.vertices);
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
            navmesher.insert_joint(JointId::new(i).into(), *joint);
        }

        for (i, segment) in board.layout().segments().collection() {
            navmesher.insert_segment(&board, SegmentId::new(i), *segment);
        }

        // TODO: Vias.

        for (i, polygon) in board.layout().polygons().collection() {
            navmesher.insert_polygon(PolygonId::new(i), polygon.clone());
        }

        Self { navmesher, board }
    }

    pub fn insert_joint(&mut self, joint: Joint) -> JointId {
        let joint_id = self.board.add_joint(joint);
        self.navmesher.insert_joint(joint_id, joint);

        joint_id
    }

    pub fn insert_segment(&mut self, segment: Segment) -> SegmentId {
        let segment_id = self.board.add_segment(segment);
        self.navmesher
            .insert_segment(&self.board, segment_id, segment);

        segment_id
    }

    pub fn insert_via(&mut self, via: Via) -> ViaId {
        // TODO: Insert into navmesh.
        self.board.add_via(via)
    }

    pub fn insert_polygon(&mut self, polygon: Polygon) -> PolygonId {
        let polygon_id = self.board.add_polygon(polygon.clone());
        self.navmesher.insert_polygon(polygon_id, polygon);

        polygon_id
    }
}
