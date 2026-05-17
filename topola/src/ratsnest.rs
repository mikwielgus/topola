// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeMap;

use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use spade::{DelaunayTriangulation, HasPosition, Triangulation, handles::FixedVertexHandle};

use crate::{
    Board, JointId, PolygonId, SegmentId, Vector2, layout::NetId, primitives::PrimitiveId,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Getters, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Ratline {
    endpoint_primitive_ids: [PrimitiveId; 2],
    endpoint_layers: [usize; 2],
    endpoints: [Vector2<i64>; 2],
}

struct DelaunayVertex {
    pub layer: usize,
    pub center: Vector2<i64>,
    pub position: spade::Point2<f64>,
    pub primitive_id: PrimitiveId,
}

impl HasPosition for DelaunayVertex {
    type Scalar = f64;

    fn position(&self) -> spade::Point2<f64> {
        self.position
    }
}

#[derive(Clone, Debug, Deserialize, Getters, Serialize)]
pub struct Ratsnest {
    ratlines: Vec<Ratline>,
}

impl Ratsnest {
    pub fn new(board: &Board) -> Self {
        let mut ratlines = Vec::new();

        let mut triangulations: BTreeMap<(NetId, usize), DelaunayTriangulation<DelaunayVertex>> =
            BTreeMap::new();

        for (i, joint) in board.layout().joints().collection() {
            let _ = triangulations
                .entry((joint.net, joint.layer))
                .or_insert_with(DelaunayTriangulation::new)
                .insert(DelaunayVertex {
                    layer: joint.layer,
                    center: joint.center(),
                    position: spade::Point2::new(joint.center().x as f64, joint.center().y as f64),
                    primitive_id: PrimitiveId::Joint(JointId::new(i)),
                });
        }

        for (i, segment) in board.layout().segments().collection() {
            let segment_center = segment.center();
            let _ = triangulations
                .entry((segment.net, segment.layer))
                .or_insert_with(DelaunayTriangulation::new)
                .insert(DelaunayVertex {
                    layer: segment.layer,
                    center: segment_center,
                    position: spade::Point2::new(segment_center.x as f64, segment_center.y as f64),
                    primitive_id: PrimitiveId::Segment(SegmentId::new(i)),
                });
        }

        for (i, polygon) in board.layout().polygons().collection() {
            let _ = triangulations
                .entry((polygon.net, polygon.layer))
                .or_insert_with(DelaunayTriangulation::new)
                .insert(DelaunayVertex {
                    layer: polygon.layer,
                    center: polygon.center(),
                    position: spade::Point2::new(
                        polygon.center().x as f64,
                        polygon.center().y as f64,
                    ),
                    primitive_id: PrimitiveId::Polygon(PolygonId::new(i)),
                });
        }

        for triangulation in triangulations.into_values() {
            if triangulation.num_vertices() < 2 {
                continue;
            }

            let mut weighted_edges: Vec<(u64, [usize; 2])> = Vec::new();

            for edge in triangulation.undirected_edges() {
                let vertices = edge.vertices();
                weighted_edges.push((
                    edge.length_2() as u64,
                    [vertices[0].index(), vertices[1].index()],
                ));
            }

            for [index0, index1] in
                crate::math::kruskal_mst(triangulation.num_vertices(), &weighted_edges)
            {
                let vertex0 = triangulation
                    .get_vertex(FixedVertexHandle::from_index(index0))
                    .unwrap();
                let vertex0 = vertex0.data();
                let vertex1 = triangulation
                    .get_vertex(FixedVertexHandle::from_index(index1))
                    .unwrap();
                let vertex1 = vertex1.data();

                ratlines.push(Ratline {
                    endpoint_primitive_ids: [vertex0.primitive_id, vertex1.primitive_id],
                    endpoint_layers: [vertex0.layer, vertex1.layer],
                    endpoints: [vertex0.center, vertex1.center],
                });
            }
        }

        Self { ratlines }
    }
}
