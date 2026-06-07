// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeMap;

use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use spade::{DelaunayTriangulation, HasPosition, Triangulation, handles::FixedVertexHandle};

use crate::{
    board::Board,
    layout::{
        LayerId,
        compounds::NetId,
        primitives::{JointId, PolyId, PrimitiveId, SegId},
    },
    vector::Vector2,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Getters, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Ratline {
    endpoint_primitive_ids: [PrimitiveId; 2],
    endpoint_layers: [LayerId; 2],
    endpoints: [Vector2<i64>; 2],
}

struct DelaunayVertex {
    pub layer: LayerId,
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

        let mut triangulations: BTreeMap<(NetId, LayerId), DelaunayTriangulation<DelaunayVertex>> =
            BTreeMap::new();

        for (i, joint) in board.layout().joints().container().iter() {
            let Some(net) = joint.spec.net else {
                continue;
            };

            let _ = triangulations
                .entry((net, joint.spec.layer))
                .or_insert_with(DelaunayTriangulation::new)
                .insert(DelaunayVertex {
                    layer: joint.spec.layer,
                    center: joint.center(),
                    position: spade::Point2::new(joint.center().x as f64, joint.center().y as f64),
                    primitive_id: PrimitiveId::Joint(JointId::new(i)),
                });
        }

        for (i, seg) in board.layout().segs().container().iter() {
            let Some(net) = seg.net else {
                continue;
            };

            let seg_center = seg.center();
            let _ = triangulations
                .entry((net, seg.layer))
                .or_insert_with(DelaunayTriangulation::new)
                .insert(DelaunayVertex {
                    layer: seg.layer,
                    center: seg_center,
                    position: spade::Point2::new(seg_center.x as f64, seg_center.y as f64),
                    primitive_id: PrimitiveId::Seg(SegId::new(i)),
                });
        }

        for (i, poly) in board.layout().polys().container().iter() {
            let Some(net) = poly.spec.net else {
                continue;
            };

            let _ = triangulations
                .entry((net, poly.spec.layer))
                .or_insert_with(DelaunayTriangulation::new)
                .insert(DelaunayVertex {
                    layer: poly.spec.layer,
                    center: poly.centroid,
                    position: spade::Point2::new(
                        poly.centroid.x as f64,
                        poly.centroid.y as f64,
                    ),
                    primitive_id: PrimitiveId::Poly(PolyId::new(i)),
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
