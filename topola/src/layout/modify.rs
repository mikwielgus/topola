// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use rstar::primitives::GeomWithData;

use crate::layout::{
    Layout,
    primitives::{
        Joint, JointId, JointSpec, Polygon, PolygonId, SegmentId, SegmentSpec, ViaId, ViaSpec,
    },
};

impl Layout {
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

    pub(super) fn modify_joint_raw<F>(&mut self, id: JointId, f: F)
    where
        F: FnOnce(&mut Joint),
    {
        let old_joint = &self.joints[id.index()];
        self.joints_rtree
            .remove(&GeomWithData::new(old_joint.bbox().rtree_rectangle(), id));

        self.joints.modify(id.index(), |joint| f(joint));

        let new_joint = self.joints[id.index()].clone();
        self.joints_rtree.insert(
            GeomWithData::new(new_joint.bbox().rtree_rectangle(), id),
            (),
        );
    }

    pub fn modify_segment<F>(&mut self, id: SegmentId, f: F)
    where
        F: FnOnce(&mut SegmentSpec),
    {
        let old_segment = &self.segments[id.index()];
        self.segments_rtree
            .remove(&GeomWithData::new(old_segment.bbox().rtree_rectangle(), id));

        self.segments
            .modify(id.index(), |segment| f(&mut segment.spec));

        let new_segment = &self.segments[id.index()];
        self.segments_rtree.insert(
            GeomWithData::new(new_segment.bbox().rtree_rectangle(), id),
            (),
        );
    }

    pub(super) fn update_segment(&mut self, id: SegmentId) {
        let old_segment = &self.segments[id.index()];
        self.segments_rtree
            .remove(&GeomWithData::new(old_segment.bbox().rtree_rectangle(), id));

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
        self.segments_rtree.insert(
            GeomWithData::new(new_segment.bbox().rtree_rectangle(), id),
            (),
        );
    }

    pub fn modify_via<F>(&mut self, id: ViaId, f: F)
    where
        F: FnOnce(&mut ViaSpec),
    {
        let old_via = &self.vias[id.index()];
        self.vias_rtree
            .remove(&GeomWithData::new(old_via.bbox().rtree_rectangle(), id));

        self.vias.modify(id.index(), |via| f(&mut via.spec));

        let new_via = &self.vias[id.index()];
        self.vias_rtree
            .insert(GeomWithData::new(new_via.bbox().rtree_rectangle(), id), ());
    }

    pub(super) fn update_via(&mut self, id: ViaId) {
        let old_via = &self.vias[id.index()];
        self.vias_rtree
            .remove(&GeomWithData::new(old_via.bbox().rtree_rectangle(), id));

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
            .insert(GeomWithData::new(new_via.bbox().rtree_rectangle(), id), ());
    }

    pub fn modify_polygon<F>(&mut self, id: PolygonId, f: F)
    where
        F: FnOnce(&mut Polygon),
    {
        let old_polygon = &self.polygons[id.index()];
        self.polygons_rtree
            .remove(&GeomWithData::new(old_polygon.bbox().rtree_rectangle(), id));

        self.polygons.modify(id.index(), |polygon| f(polygon));

        let new_polygon = &self.polygons[id.index()];
        self.polygons_rtree.insert(
            GeomWithData::new(new_polygon.bbox().rtree_rectangle(), id),
            (),
        );
    }
}
