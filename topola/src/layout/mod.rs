// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

pub mod primitives;
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

use crate::{
    compounds::{Component, ComponentId, Pin, PinId},
    layout::primitives::{
        Joint, JointId, JointSpec, Polygon, PolygonId, Segment, SegmentId, SegmentSpec, Via, ViaId,
        ViaSpec,
    },
    math::Vector2,
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

    pub fn add_component(&mut self) -> ComponentId {
        ComponentId::new(self.components.push(Component::new()))
    }

    pub fn add_pin(&mut self) -> PinId {
        PinId::new(self.pins.push(Pin::new()))
    }

    pub fn add_joint(&mut self, spec: JointSpec) -> JointId {
        let joint = Joint {
            spec,
            segments: Vec::new(),
            vias: Vec::new(),
        };
        let bbox = joint.bbox();
        let component_id = joint.spec.component;
        let pin_id = joint.spec.pin;
        let joint_id = JointId::new(self.joints.push(joint));

        self.joints_rtree
            .insert(GeomWithData::new(bbox, joint_id), ());

        if let Some(component_id) = component_id {
            self.components.modify(component_id.index(), |component| {
                component.joints.push(joint_id)
            });
        }

        if let Some(pin_id) = pin_id {
            self.pins
                .modify(pin_id.index(), |pin| pin.joints.push(joint_id));
        }

        joint_id
    }

    pub fn modify_joint<F>(&mut self, id: JointId, f: F)
    where
        F: FnOnce(&mut JointSpec),
    {
        self.modify_joint_raw(id, |joint| f(&mut joint.spec));
        let new_joint = self.joints[id.index()].clone();

        for &segment_id in &new_joint.segments {
            self.update_segment(segment_id);
        }

        for &via_id in &new_joint.vias {
            self.update_via(via_id);
        }
    }

    fn modify_joint_raw<F>(&mut self, id: JointId, f: F)
    where
        F: FnOnce(&mut Joint),
    {
        let old_joint = &self.joints[id.index()];
        self.joints_rtree
            .remove(&GeomWithData::new(old_joint.bbox(), id));

        self.joints.modify(id.index(), |joint| f(joint));

        let new_joint = self.joints[id.index()].clone();
        self.joints_rtree
            .insert(GeomWithData::new(new_joint.bbox(), id), ());
    }

    pub fn add_segment(&mut self, spec: SegmentSpec) -> SegmentId {
        self.add_segment_raw(Segment {
            spec,
            endpoints: [
                self.joint(spec.endjoints[0]).spec.position,
                self.joint(spec.endjoints[1]).spec.position,
            ],
            layer: self.joint(spec.endjoints[0]).spec.layer,
            net: self.joint(spec.endjoints[0]).spec.net,
        })
    }

    pub fn add_segment_raw(&mut self, segment: Segment) -> SegmentId {
        let component_id = segment.spec.component;
        let pin_id = segment.spec.pin;
        let bbox = segment.bbox();
        let segment_id = SegmentId::new(self.segments.push(segment));

        self.joints
            .modify(segment.spec.endjoints[0].index(), |joint| {
                joint.segments.push(segment_id)
            });
        self.joints
            .modify(segment.spec.endjoints[1].index(), |joint| {
                joint.segments.push(segment_id)
            });

        self.segments_rtree
            .insert(GeomWithData::new(bbox, segment_id), ());

        if let Some(component_id) = component_id {
            self.components.modify(component_id.index(), |component| {
                component.segments.push(segment_id)
            });
        }

        if let Some(pin_id) = pin_id {
            self.pins
                .modify(pin_id.index(), |pin| pin.segments.push(segment_id));
        }

        segment_id
    }

    pub fn modify_segment<F>(&mut self, id: SegmentId, f: F)
    where
        F: FnOnce(&mut SegmentSpec),
    {
        let old_segment = &self.segments[id.index()];
        self.segments_rtree
            .remove(&GeomWithData::new(old_segment.bbox(), id));

        self.segments
            .modify(id.index(), |segment| f(&mut segment.spec));

        let new_segment = &self.segments[id.index()];
        self.segments_rtree
            .insert(GeomWithData::new(new_segment.bbox(), id), ());
    }

    fn update_segment(&mut self, id: SegmentId) {
        let old_segment = &self.segments[id.index()];
        self.segments_rtree
            .remove(&GeomWithData::new(old_segment.bbox(), id));

        let endjoint_ids = old_segment.spec.endjoints;
        let endjoint_specs = [
            self.joints[endjoint_ids[0].index()].spec,
            self.joints[endjoint_ids[1].index()].spec,
        ];
        self.segments.modify(id.index(), |segment| {
            segment.endpoints = [endjoint_specs[0].position, endjoint_specs[1].position];
            segment.layer = endjoint_specs[0].layer;
            segment.net = endjoint_specs[0].net;
        });

        let new_segment = &self.segments[id.index()];
        self.segments_rtree
            .insert(GeomWithData::new(new_segment.bbox(), id), ());
    }

    pub fn add_via(&mut self, spec: ViaSpec) -> ViaId {
        let joints = [self.joint(spec.endjoints[0]), self.joint(spec.endjoints[1])];

        self.add_via_raw(Via {
            spec,
            position: joints[0].spec.position,
            min_layer: std::cmp::min(joints[0].spec.layer, joints[1].spec.layer),
            max_layer: std::cmp::max(joints[0].spec.layer, joints[1].spec.layer),
            net: joints[0].spec.net,
        })
    }

    pub fn add_via_raw(&mut self, via: Via) -> ViaId {
        let bbox = via.bbox();
        let component_id = via.spec.component;
        let pin_id = via.spec.pin;
        let via_id = ViaId::new(self.vias.push(via));

        self.joints.modify(via.spec.endjoints[0].index(), |joint| {
            joint.vias.push(via_id)
        });
        self.joints.modify(via.spec.endjoints[1].index(), |joint| {
            joint.vias.push(via_id)
        });

        self.vias_rtree.insert(GeomWithData::new(bbox, via_id), ());

        if let Some(component_id) = component_id {
            self.components.modify(component_id.index(), |component| {
                component.vias.push(via_id)
            });
        }

        if let Some(pin_id) = pin_id {
            self.pins
                .modify(pin_id.index(), |pin| pin.vias.push(via_id));
        }

        via_id
    }

    pub fn modify_via<F>(&mut self, id: ViaId, f: F)
    where
        F: FnOnce(&mut ViaSpec),
    {
        let old_via = &self.vias[id.index()];
        self.vias_rtree
            .remove(&GeomWithData::new(old_via.bbox(), id));

        self.vias.modify(id.index(), |via| f(&mut via.spec));

        let new_via = &self.vias[id.index()];
        self.vias_rtree
            .insert(GeomWithData::new(new_via.bbox(), id), ());
    }

    fn update_via(&mut self, id: ViaId) {
        let old_via = &self.vias[id.index()];
        self.vias_rtree
            .remove(&GeomWithData::new(old_via.bbox(), id));

        let endjoint_ids = old_via.spec.endjoints;
        let endjoint_specs = [
            self.joints[endjoint_ids[0].index()].spec,
            self.joints[endjoint_ids[1].index()].spec,
        ];
        self.vias.modify(id.index(), |via| {
            via.position = endjoint_specs[0].position;
            via.min_layer = std::cmp::min(endjoint_specs[0].layer, endjoint_specs[1].layer);
            via.max_layer = std::cmp::max(endjoint_specs[0].layer, endjoint_specs[1].layer);
            via.net = endjoint_specs[0].net;
        });

        let new_via = &self.vias[id.index()];
        self.vias_rtree
            .insert(GeomWithData::new(new_via.bbox(), id), ());
    }

    pub fn add_polygon(&mut self, polygon: Polygon) -> PolygonId {
        let bbox = polygon.bbox();
        let component_id = polygon.component;
        let pin_id = polygon.pin;
        let polygon_id = PolygonId::new(self.polygons.push(polygon));

        self.polygons_rtree
            .insert(GeomWithData::new(bbox, polygon_id), ());

        if let Some(component_id) = component_id {
            self.components.modify(component_id.index(), |component| {
                component.polygons.push(polygon_id)
            });
        }

        if let Some(pin_id) = pin_id {
            self.pins
                .modify(pin_id.index(), |pin| pin.polygons.push(polygon_id));
        }

        polygon_id
    }

    pub fn modify_polygon<F>(&mut self, id: PolygonId, f: F)
    where
        F: FnOnce(&mut Polygon),
    {
        let old_polygon = &self.polygons[id.index()];
        self.polygons_rtree
            .remove(&GeomWithData::new(old_polygon.bbox(), id));

        self.polygons.modify(id.index(), |polygon| f(polygon));

        let new_polygon = &self.polygons[id.index()];
        self.polygons_rtree
            .insert(GeomWithData::new(new_polygon.bbox(), id), ());
    }

    pub fn locate_joints_at_point(
        &self,
        layer: LayerId,
        point: Vector2<i64>,
    ) -> impl Iterator<Item = JointId> {
        self.joints_rtree
            .as_ref()
            .locate_all_at_point(&[point.x, point.y, layer.index() as i64])
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&joint_id| self.joints[joint_id.index()].contains_point(point))
    }

    pub fn locate_segments_at_point(
        &self,
        layer: LayerId,
        point: Vector2<i64>,
    ) -> impl Iterator<Item = SegmentId> {
        self.segments_rtree
            .as_ref()
            .locate_all_at_point(&[point.x, point.y, layer.index() as i64])
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&segment_id| self.segment(segment_id).contains_point(point))
    }

    // TODO: vias.

    pub fn locate_polygons_at_point(
        &self,
        layer: LayerId,
        point: Vector2<i64>,
    ) -> impl Iterator<Item = PolygonId> {
        self.polygons_rtree
            .as_ref()
            .locate_all_at_point(&[point.x, point.y, layer.index() as i64])
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&polygon_id| self.polygons[polygon_id.index()].contains_point(point))
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

    pub fn joint(&self, joint_id: JointId) -> &Joint {
        &self.joints[joint_id.index()]
    }

    pub fn segment(&self, segment_id: SegmentId) -> &Segment {
        &self.segments[segment_id.index()]
    }

    pub fn polygon(&self, polygon_id: PolygonId) -> &Polygon {
        &self.polygons[polygon_id.index()]
    }

    pub fn pin(&self, pin_id: PinId) -> &Pin {
        &self.pins[pin_id.index()]
    }
}
