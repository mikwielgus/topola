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
    board::Board,
    layout::{
        LayerId,
        primitives::{Joint, JointId, JointSpec, Poly, PolyId, Seg, SegId},
    },
    vector::Vector2,
};

#[derive(
    Clone,
    Constructor,
    Copy,
    Debug,
    Default,
    Deserialize,
    Eq,
    From,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
)]
pub struct MultiObstacleId {
    layer: LayerId,
    index: usize,
}

impl MultiObstacleId {
    /// Layer of the obstacle.
    #[inline]
    pub fn layer(self) -> LayerId {
        self.layer
    }

    /// Index of the obstacle on the navmesh at its layer.
    #[inline]
    pub fn index(self) -> usize {
        self.index
    }
}

#[derive(
    Clone, Constructor, Debug, Default, Deserialize, Eq, From, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct MultiVertexId {
    layer: LayerId,
    indices: Vec<VertexId>,
}

impl MultiVertexId {
    /// Layer of the obstacle.
    #[inline]
    pub fn layer(self) -> LayerId {
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

    pub fn insert_multiobstacle(&mut self, poly: impl IntoIterator<Item = Vector2<i64>>) -> usize {
        let poly: Vec<Vector2<i64>> = poly.into_iter().collect();
        let mut index = 0;

        for i in 0..self.navmeshes.len() {
            index = self.navmeshes[i].insert_obstacle_and_rebuild(
                Self::inflate_poly(poly.clone(), self.inflation_factors[i])
                    .into_iter()
                    .map(Into::into),
                self.boundary.iter().cloned().map(Into::into),
            );
        }

        index
    }

    fn inflate_poly(
        poly: Vec<Vector2<i64>>,
        inflation_factor: f64,
    ) -> impl IntoIterator<Item = Vector2<i64>> {
        // Centroid.
        let cx = poly.iter().map(|p| p.x as f64).sum::<f64>() / poly.len() as f64;
        let cy = poly.iter().map(|p| p.y as f64).sum::<f64>() / poly.len() as f64;

        poly.into_iter().map(move |p| {
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
        layer: LayerId,
        multiobstacle: impl IntoIterator<Item = Vector2<i64>>,
    ) -> MultiObstacleId {
        MultiObstacleId::new(
            layer,
            self.layer_navmeshers[layer.index()].insert_multiobstacle(multiobstacle),
        )
    }

    pub fn insert_free_multivertex_in_multiobstacle(
        &mut self,
        multiobstacle_id: MultiObstacleId,
        position: Vector2<i64>,
    ) -> MultiVertexId {
        MultiVertexId {
            layer: multiobstacle_id.layer,
            indices: self.layer_navmeshers[multiobstacle_id.layer.index()]
                .insert_free_multivertex_in_multiobstacle(multiobstacle_id.index, position),
        }
    }
}

#[derive(Clone, Debug, Getters)]
pub struct NavmesherBoard {
    navmesher: Navmesher,
    board: Board,

    joint_multiobstacles: Recorder<StableVec<MultiObstacleId>>,
    seg_multiobstacles: Recorder<StableVec<MultiObstacleId>>,
    poly_multiobstacles: Recorder<StableVec<MultiObstacleId>>,
}

impl NavmesherBoard {
    pub fn new(board: Board) -> Self {
        let mut this = Self {
            navmesher: Navmesher::new(
                board.layout().boundary().iter().copied(),
                *board.layout().layer_count(),
            ),
            board,

            joint_multiobstacles: Recorder::new(StableVec::new()),
            seg_multiobstacles: Recorder::new(StableVec::new()),
            poly_multiobstacles: Recorder::new(StableVec::new()),
        };

        /*for (i, joint) in this.board.layout().joints().container().iter() {
            this.joint_multiobstacles.insert(
                i,
                this.navmesher
                    .insert_multiobstacle(joint.spec.layer, Self::joint_bounding_octagon(joint)),
            );
        }

        for (i, seg) in this.board.layout().segs().container().iter() {
            this.seg_multiobstacles.insert(
                i,
                this.navmesher
                    .insert_multiobstacle(seg.layer, seg.bounding_rectangle()),
            );
        }

        for (i, poly) in this.board.layout().polys().container().iter() {
            this.poly_multiobstacles.insert(
                i,
                this.navmesher
                    .insert_multiobstacle(poly.spec.layer, poly.spec.vertices.clone()),
            );
        }*/

        this
    }

    pub fn insert_joint(&mut self, spec: JointSpec) -> JointId {
        let layer = spec.layer;
        let obstacle = Self::joint_bounding_octagon(&Joint::new(spec));
        let joint_id = self.board.insert_joint(spec);
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

    pub fn insert_seg_with_cache(&mut self, seg: Seg) -> SegId {
        let layer = seg.layer;
        let obstacle = seg.bounding_rectangle();
        let seg_id = self.board.insert_seg_raw(seg);
        self.seg_multiobstacles.insert(
            seg_id.index(),
            self.navmesher.insert_multiobstacle(layer, obstacle),
        );

        seg_id
    }

    pub fn insert_poly(&mut self, poly: Poly) -> PolyId {
        let poly_id = self.board.insert_poly(poly.clone());
        self.poly_multiobstacles.insert(
            poly_id.index(),
            self.navmesher
                .insert_multiobstacle(poly.spec.layer, poly.spec.vertices),
        );

        poly_id
    }
}
