// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use spade::{DelaunayTriangulation, HasPosition, Triangulation, handles::FixedVertexHandle};

use crate::{Board, JointId, PolygonId, SegmentId, Vector2, primitives::PrimitiveId};

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

        for layer in 0..*board.layout().layer_count() {
            let mut triangulation: DelaunayTriangulation<DelaunayVertex> =
                DelaunayTriangulation::new();

            for layer in 0..*board.layout().layer_count() {
                for (i, joint) in board.layout().joints().collection() {
                    if joint.layer == layer {
                        triangulation.insert(DelaunayVertex {
                            layer,
                            center: joint.center(),
                            position: spade::Point2::new(
                                joint.center().x as f64,
                                joint.center().y as f64,
                            ),
                            primitive_id: PrimitiveId::Joint(JointId::new(i)),
                        });
                    }
                }

                for (i, segment) in board.layout().segments().collection() {
                    if segment.layer == layer {
                        let segment_center = board.layout().segment_center(SegmentId::new(i));
                        triangulation.insert(DelaunayVertex {
                            layer,
                            center: segment_center,
                            position: spade::Point2::new(
                                segment_center.x as f64,
                                segment_center.y as f64,
                            ),
                            primitive_id: PrimitiveId::Segment(SegmentId::new(i)),
                        });
                    }
                }

                for (i, polygon) in board.layout().polygons().collection() {
                    if polygon.layer == layer {
                        triangulation.insert(DelaunayVertex {
                            layer,
                            center: polygon.center(),
                            position: spade::Point2::new(
                                polygon.center().x as f64,
                                polygon.center().y as f64,
                            ),
                            primitive_id: PrimitiveId::Polygon(PolygonId::new(i)),
                        });
                    }
                }
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
