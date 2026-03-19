// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use dearcut::{RecordingTriangulator, VertexId};
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
pub struct MultiObstacleId {
    layer: usize,
    index: usize,
}

impl MultiObstacleId {
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

#[derive(Clone, Constructor, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct MultiVertexId {
    layer: usize,
    indices: Vec<VertexId>,
}

impl MultiVertexId {
    /// Layer of the obstacle.
    #[inline]
    pub fn layer(self) -> usize {
        self.layer
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

    pub fn insert_multiobstacle(
        &mut self,
        polygon: impl IntoIterator<Item = Vector2<i64>>,
    ) -> usize {
        let polygon: Vec<Vector2<i64>> = polygon.into_iter().collect();
        let mut index = 0;

        for i in 0..self.navmeshes.len() {
            index = self.navmeshes[i].insert_obstacle_and_rebuild(
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

    pub fn insert_free_multivertex_in_multiobstacle(
        &mut self,
        multiobstacle_index: usize,
        position: Vector2<i64>,
    ) -> Vec<VertexId> {
        let mut vertices = Vec::new();

        for navmesh in &mut self.navmeshes {
            vertices.push(
                navmesh
                    .insert_free_vertex_in_obstacle(multiobstacle_index, [position.x, position.y]),
            );
        }

        vertices
    }
}

#[derive(Clone, Debug, Getters)]
pub struct Navmesher {
    layer_navmeshers: Vec<LayerNavmesher>,
}

impl Navmesher {
    pub fn new(boundary: impl IntoIterator<Item = Vector2<i64>>, layer_count: usize) -> Self {
        let boundary: Vec<Vector2<i64>> = boundary.into_iter().collect();

        Self {
            layer_navmeshers: std::iter::repeat_with(|| LayerNavmesher::new(boundary.clone()))
                .take(layer_count)
                .collect(),
        }
    }

    pub fn insert_multiobstacle(
        &mut self,
        layer: usize,
        multiobstacle: impl IntoIterator<Item = Vector2<i64>>,
    ) -> MultiObstacleId {
        MultiObstacleId::new(
            layer,
            self.layer_navmeshers[layer].insert_multiobstacle(multiobstacle),
        )
    }

    pub fn insert_free_multivertex_in_multiobstacle(
        &mut self,
        multiobstacle_id: MultiObstacleId,
        position: Vector2<i64>,
    ) -> MultiVertexId {
        MultiVertexId {
            layer: multiobstacle_id.layer,
            indices: self.layer_navmeshers[multiobstacle_id.layer]
                .insert_free_multivertex_in_multiobstacle(multiobstacle_id.index, position),
        }
    }
}

#[derive(Clone, Debug, Getters)]
pub struct NavmesherBoard {
    navmesher: Navmesher,
    board: Board,

    joint_multiobstacles: Recorder<Vec<MultiObstacleId>>,
    segment_multiobstacles: Recorder<Vec<MultiObstacleId>>,
    polygon_multiobstacles: Recorder<Vec<MultiObstacleId>>,
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
        self.polygon_multiobstacles.insert(
            polygon_id.index(),
            self.navmesher
                .insert_multiobstacle(polygon.layer, polygon.vertices),
        );

        polygon_id
    }
}
