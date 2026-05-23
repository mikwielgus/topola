// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use dearcut::{RecordingTriangulator, VertexId};
use derive_getters::Getters;
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};
use stable_vec::StableVec;
use undoredo::Recorder;

use crate::{
    Board,
    math::Vector2,
    primitives::{Joint, JointId, JointSpec, Polygon, PolygonId, Segment, SegmentId},
};

#[derive(
    Clone, Constructor, Copy, Debug, Deserialize, Eq, From, Ord, PartialEq, PartialOrd, Serialize,
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

#[derive(
    Clone, Constructor, Debug, Deserialize, Eq, From, Ord, PartialEq, PartialOrd, Serialize,
)]
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

    joint_multiobstacles: Recorder<StableVec<MultiObstacleId>>,
    segment_multiobstacles: Recorder<StableVec<MultiObstacleId>>,
    polygon_multiobstacles: Recorder<StableVec<MultiObstacleId>>,
}

impl NavmesherBoard {
    pub fn new(board: Board) -> Self {
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

            joint_multiobstacles: Recorder::new(StableVec::new()),
            segment_multiobstacles: Recorder::new(StableVec::new()),
            polygon_multiobstacles: Recorder::new(StableVec::new()),
        };

        for (i, joint) in this.board.layout().joints().container().iter() {
            this.joint_multiobstacles.insert(
                i,
                this.navmesher
                    .insert_multiobstacle(joint.spec.layer, Self::joint_bounding_octagon(joint)),
            );
        }

        for (i, segment) in this.board.layout().segments().container().iter() {
            this.segment_multiobstacles.insert(
                i,
                this.navmesher
                    .insert_multiobstacle(segment.layer, segment.bounding_rectangle()),
            );
        }

        for (i, polygon) in this.board.layout().polygons().container().iter() {
            this.polygon_multiobstacles.insert(
                i,
                this.navmesher
                    .insert_multiobstacle(polygon.layer, polygon.vertices.clone()),
            );
        }

        this
    }

    pub fn insert_joint(&mut self, spec: JointSpec) -> JointId {
        let layer = spec.layer;
        let obstacle = Self::joint_bounding_octagon(&Joint {
            spec,
            segments: Vec::new(),
            vias: Vec::new(),
        });
        let joint_id = self.board.add_joint(spec);
        self.joint_multiobstacles.insert(
            joint_id.index(),
            self.navmesher.insert_multiobstacle(layer, obstacle),
        );

        joint_id
    }

    fn joint_bounding_octagon(joint: &Joint) -> [Vector2<i64>; 8] {
        let cx = joint.spec.position.x;
        let cy = joint.spec.position.y;
        let r = joint.spec.radius as i64;

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

    pub fn insert_segment_with_cache(&mut self, segment: Segment) -> SegmentId {
        let layer = segment.layer;
        let obstacle = segment.bounding_rectangle();
        let segment_id = self.board.add_segment_raw(segment);
        self.segment_multiobstacles.insert(
            segment_id.index(),
            self.navmesher.insert_multiobstacle(layer, obstacle),
        );

        segment_id
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
