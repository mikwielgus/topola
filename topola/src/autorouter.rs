// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_getters::Getters;
use undoredo::Recorder;

use crate::{
    Board, Joint, JointId, Polygon, PolygonId, Segment, SegmentId, Vector2, Via, ViaId,
    navmesher::{MultiObstacleId, Navmesher},
};

#[derive(Clone, Debug, Getters)]
pub struct Autorouter {
    navmesher: Navmesher,
    board: Board,

    joint_multiobstacles: Recorder<Vec<MultiObstacleId>>,
    segment_multiobstacles: Recorder<Vec<MultiObstacleId>>,
    polygon_multiobstacles: Recorder<Vec<MultiObstacleId>>,
}

impl Autorouter {
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

            joint_multiobstacles: Recorder::new(Vec::new()),
            segment_multiobstacles: Recorder::new(Vec::new()),
            polygon_multiobstacles: Recorder::new(Vec::new()),
        };

        for (i, joint) in this.board.layout().joints().collection() {
            this.joint_multiobstacles.insert(
                i,
                this.navmesher
                    .insert_multiobstacle(joint.layer, Self::joint_bounding_octagon(*joint)),
            );
        }

        for (i, segment) in this.board.layout().segments().collection() {
            this.segment_multiobstacles.insert(
                i,
                this.navmesher.insert_multiobstacle(
                    segment.layer,
                    this.segment_bounding_rectangle(SegmentId::new(i), *segment),
                ),
            );
        }

        for (i, polygon) in this.board.layout().polygons().collection() {
            this.polygon_multiobstacles.insert(
                i,
                this.navmesher
                    .insert_multiobstacle(polygon.layer, polygon.vertices.clone()),
            );
        }

        this
    }

    pub fn insert_joint(&mut self, joint: Joint) -> JointId {
        let joint_id = self.board.add_joint(joint);
        self.joint_multiobstacles.insert(
            joint_id.index(),
            self.navmesher
                .insert_multiobstacle(joint.layer, Self::joint_bounding_octagon(joint)),
        );

        joint_id
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
        self.segment_multiobstacles.insert(
            segment_id.index(),
            self.navmesher.insert_multiobstacle(
                segment.layer,
                self.segment_bounding_rectangle(segment_id, segment),
            ),
        );

        segment_id
    }

    fn segment_bounding_rectangle(
        &self,
        segment_id: SegmentId,
        segment: Segment,
    ) -> [Vector2<i64>; 4] {
        let endpoints = self.board.layout().segment_endpoints(segment_id);
        crate::math::inflated_segment(
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
        let polygon_id = self.board.add_polygon(polygon.clone());
        self.polygon_multiobstacles.insert(
            polygon_id.index(),
            self.navmesher
                .insert_multiobstacle(polygon.layer, polygon.vertices),
        );

        polygon_id
    }
}
