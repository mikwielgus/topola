// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use dearcut::RecordingTriangulator;
use derive_getters::Getters;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use undoredo::Recorder;

use crate::{
    Board, Vector2, math,
    primitives::{Joint, JointId, Polygon, PolygonId, Segment, SegmentId, Via, ViaId},
};

#[derive(
    Clone, Constructor, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct ObstacleId {
    layer: usize,
    index: usize,
}

impl ObstacleId {
    /// Layer of the obstacle.
    #[inline]
    pub fn layer(self) -> usize {
        self.layer
    }

    /// Index of the obstacle on the navmesh at its layer.
    #[inline]
    pub fn index(self) -> usize {
        self.index
    }
}

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

    pub fn insert_obstacle(&mut self, polygon: impl IntoIterator<Item = Vector2<i64>>) -> usize {
        let polygon: Vec<Vector2<i64>> = polygon.into_iter().collect();
        let mut index = 0;

        for i in 0..self.navmeshes.len() {
            index = self.navmeshes[i].insert_polygon_and_rebuild(
                Self::inflate_polygon(polygon.clone(), self.inflation_factors[i])
                    .into_iter()
                    .map(Into::into),
                self.boundary.iter().cloned().map(Into::into),
            );
        }

        index
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

    pub fn insert_obstacle(
        &mut self,
        layer: usize,
        polygon: impl IntoIterator<Item = Vector2<i64>>,
    ) -> ObstacleId {
        ObstacleId::new(layer, self.layers[layer].insert_obstacle(polygon))
    }
}

#[derive(Clone, Debug, Getters)]
pub struct NavmesherBoard {
    navmesher: Navmesher,
    board: Board,

    joint_obstacles: Recorder<Vec<ObstacleId>>,
    segment_obstacles: Recorder<Vec<ObstacleId>>,
    polygon_obstacles: Recorder<Vec<ObstacleId>>,
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

            joint_obstacles: Recorder::new(Vec::new()),
            segment_obstacles: Recorder::new(Vec::new()),
            polygon_obstacles: Recorder::new(Vec::new()),
        };

        for (i, joint) in this.board.layout().joints().collection() {
            this.joint_obstacles.insert(
                i,
                this.navmesher
                    .insert_obstacle(joint.layer, Self::joint_bounding_octagon(*joint)),
            );
        }

        for (i, segment) in this.board.layout().segments().collection() {
            this.segment_obstacles.insert(
                i,
                this.navmesher.insert_obstacle(
                    segment.layer,
                    this.segment_bounding_rectangle(SegmentId::new(i), *segment),
                ),
            );
        }

        for (i, polygon) in this.board.layout().polygons().collection() {
            this.polygon_obstacles.insert(
                i,
                this.navmesher
                    .insert_obstacle(polygon.layer, polygon.vertices.clone()),
            );
        }

        this
    }

    pub fn insert_joint(&mut self, joint: Joint) -> JointId {
        let joint_id = self.board.add_joint(joint);
        self.joint_obstacles.insert(
            joint_id.id(),
            self.navmesher
                .insert_obstacle(joint.layer, Self::joint_bounding_octagon(joint)),
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
        self.segment_obstacles.insert(
            segment_id.id(),
            self.navmesher.insert_obstacle(
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
        let polygon_id = self.board.add_polygon(polygon.clone());
        self.polygon_obstacles.insert(
            polygon_id.id(),
            self.navmesher
                .insert_obstacle(polygon.layer, polygon.vertices),
        );

        polygon_id
    }
}
