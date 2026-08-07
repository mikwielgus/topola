// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use rstar::primitives::GeomWithData;

use crate::layout::{
    Layout,
    primitives::{
        Joint, JointId, JointSpec, Poly, PolyId, Seg, SegId, SegSpec, Via, ViaId, ViaSpec,
    },
};

impl Layout {
    pub fn modify_joint<F>(&mut self, id: JointId, f: F)
    where
        F: FnOnce(&mut JointSpec),
    {
        self.modify_joint_raw(id, |joint| f(&mut joint.spec));
        let new_joint = self.joints[id.index()].clone();

        for &seg_id in &new_joint.segs {
            self.update_seg(seg_id);
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

    pub fn modify_seg<F>(&mut self, id: SegId, f: F)
    where
        F: FnOnce(&mut SegSpec),
    {
        let old_seg = &self.segs[id.index()];
        self.segs_rtree
            .remove(&GeomWithData::new(old_seg.bbox().rtree_rectangle(), id));

        self.segs.modify(id.index(), |seg| f(&mut seg.spec));

        let new_seg = &self.segs[id.index()];
        self.segs_rtree
            .insert(GeomWithData::new(new_seg.bbox().rtree_rectangle(), id), ());
    }

    pub(super) fn update_seg(&mut self, id: SegId) {
        let old_seg = &self.segs[id.index()];
        self.segs_rtree
            .remove(&GeomWithData::new(old_seg.bbox().rtree_rectangle(), id));

        let endjoint_ids = old_seg.spec.endjoints;
        let endjoints = [
            &self.joints[endjoint_ids[0].index()],
            &self.joints[endjoint_ids[1].index()],
        ];
        let spec = old_seg.spec;
        self.segs.modify(id.index(), |seg| {
            *seg = Seg::new(spec, endjoints);
        });

        let new_seg = &self.segs[id.index()];
        self.segs_rtree
            .insert(GeomWithData::new(new_seg.bbox().rtree_rectangle(), id), ());
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
        let endjoints = [
            &self.joints[endjoint_ids[0].index()],
            &self.joints[endjoint_ids[1].index()],
        ];
        let spec = old_via.spec;
        self.vias.modify(id.index(), |via| {
            *via = Via::new(spec, endjoints);
        });

        let new_via = &self.vias[id.index()];
        self.vias_rtree
            .insert(GeomWithData::new(new_via.bbox().rtree_rectangle(), id), ());
    }

    pub fn modify_poly<F>(&mut self, id: PolyId, f: F)
    where
        F: FnOnce(&mut Poly),
    {
        let old_poly = &self.polys[id.index()];
        self.polys_rtree
            .remove(&GeomWithData::new(old_poly.bbox().rtree_rectangle(), id));

        self.polys.modify(id.index(), |poly| f(poly));

        let new_poly = &self.polys[id.index()];
        self.polys_rtree
            .insert(GeomWithData::new(new_poly.bbox().rtree_rectangle(), id), ());
    }
}
