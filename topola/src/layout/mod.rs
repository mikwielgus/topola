// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

mod bbox;
pub mod compounds;
mod delete;
mod infringement;
mod insert;
mod locate;
mod modify;
mod overlap;
pub mod primitives;
mod repulsion;
mod transforms;

use derive_getters::Getters;
use derive_more::{Constructor, From};
use rstar::{
    AABB, RTree,
    primitives::{GeomWithData, Rectangle},
};
use serde::{Deserialize, Serialize};
use stable_vec::StableVec;
use undoredo::aliases::RTreeHalfDelta;
use undoredo::{Delta, Recorder};

use crate::layout::{
    compounds::{Component, ComponentId, Net, Pin, PinId},
    primitives::{Joint, JointId, Polygon, PolygonId, Segment, SegmentId, Via, ViaId},
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
pub struct LayerId(usize);

impl LayerId {
    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug, Delta, Getters)]
pub struct Layout {
    #[undoredo(skip)]
    boundary: Vec<[i64; 2]>,
    #[undoredo(skip)]
    place_boundary: Vec<[i64; 2]>,
    #[undoredo(skip)]
    layer_count: usize,

    components: Recorder<StableVec<Component>>,
    nets: Recorder<StableVec<Net>>,
    pins: Recorder<StableVec<Pin>>,

    joints: Recorder<StableVec<Joint>>,
    segments: Recorder<StableVec<Segment>>,
    vias: Recorder<StableVec<Via>>,
    polygons: Recorder<StableVec<Polygon>>,

    joints_rtree: Recorder<
        RTree<GeomWithData<Rectangle<[i64; 3]>, JointId>>,
        RTreeHalfDelta<GeomWithData<Rectangle<[i64; 3]>, JointId>>,
    >,
    segments_rtree: Recorder<
        RTree<GeomWithData<Rectangle<[i64; 3]>, SegmentId>>,
        RTreeHalfDelta<GeomWithData<Rectangle<[i64; 3]>, SegmentId>>,
    >,
    vias_rtree: Recorder<
        RTree<GeomWithData<Rectangle<[i64; 3]>, ViaId>>,
        RTreeHalfDelta<GeomWithData<Rectangle<[i64; 3]>, ViaId>>,
    >,
    polygons_rtree: Recorder<
        RTree<GeomWithData<Rectangle<[i64; 3]>, PolygonId>>,
        RTreeHalfDelta<GeomWithData<Rectangle<[i64; 3]>, PolygonId>>,
    >,
}

impl Layout {
    pub fn new(boundary: Vec<[i64; 2]>, layer_count: usize) -> Self {
        Self {
            boundary: boundary.clone(),
            place_boundary: boundary,
            layer_count,

            components: Recorder::new(StableVec::new()),
            nets: Recorder::new(StableVec::new()),
            pins: Recorder::new(StableVec::new()),

            joints: Recorder::new(StableVec::new()),
            segments: Recorder::new(StableVec::new()),
            vias: Recorder::new(StableVec::new()),
            polygons: Recorder::new(StableVec::new()),

            joints_rtree: Recorder::new(RTree::new()),
            segments_rtree: Recorder::new(RTree::new()),
            vias_rtree: Recorder::new(RTree::new()),
            polygons_rtree: Recorder::new(RTree::new()),
        }
    }

    pub fn layer_joints(&self, layer: LayerId) -> impl Iterator<Item = JointId> + '_ {
        let envelope = Self::whole_layer_aabb(layer);
        self.joints_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&envelope)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&id| self.joint(id).spec.layer == layer)
    }

    pub fn layer_segments(&self, layer: LayerId) -> impl Iterator<Item = SegmentId> + '_ {
        let envelope = Self::whole_layer_aabb(layer);
        self.segments_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&envelope)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&id| self.segment(id).layer == layer)
    }

    pub fn layer_vias(&self, layer: LayerId) -> impl Iterator<Item = ViaId> + '_ {
        let envelope = Self::whole_layer_aabb(layer);
        self.vias_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&envelope)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&id| {
                let via = self.via(id);
                via.min_layer <= layer && layer <= via.max_layer
            })
    }

    pub fn layer_polygons(&self, layer: LayerId) -> impl Iterator<Item = PolygonId> + '_ {
        let envelope = Self::whole_layer_aabb(layer);
        self.polygons_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&envelope)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&id| self.polygon(id).layer == layer)
    }

    fn whole_layer_aabb(layer: LayerId) -> AABB<[i64; 3]> {
        AABB::from_corners(
            [i64::MIN, i64::MIN, layer.index() as i64],
            [i64::MAX, i64::MAX, layer.index() as i64],
        )
    }

    pub fn component(&self, component_id: ComponentId) -> &Component {
        &self.components[component_id.index()]
    }

    pub fn pin(&self, pin_id: PinId) -> &Pin {
        &self.pins[pin_id.index()]
    }

    pub fn joint(&self, joint_id: JointId) -> &Joint {
        &self.joints[joint_id.index()]
    }

    pub fn segment(&self, segment_id: SegmentId) -> &Segment {
        &self.segments[segment_id.index()]
    }

    pub fn via(&self, via_id: ViaId) -> &Via {
        &self.vias[via_id.index()]
    }

    pub fn polygon(&self, polygon_id: PolygonId) -> &Polygon {
        &self.polygons[polygon_id.index()]
    }
}
