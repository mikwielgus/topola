// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use derive_more::Constructor;
use rstar::{
    AABB, RTree,
    primitives::{GeomWithData, Rectangle},
};
use serde::{Deserialize, Serialize};

use crate::layout::primitives::PrimitiveId;

use super::Layout;
use super::compounds::{ComponentId, NetId, PinId};
use super::primitives::{JointId, PolyId, SegId, ViaId};

#[derive(
    Clone, Copy, Constructor, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct Infringement<T = PrimitiveId, U = PrimitiveId> {
    infringer: T,
    infringee: U,
}

impl<T: Copy, U: Copy> Infringement<T, U> {
    pub fn infringer(&self) -> T {
        self.infringer
    }

    pub fn infringee(&self) -> U {
        self.infringee
    }
}

mod sealed {
    use super::*;

    // Sealed trait to prevent having our `From` implementations below collide
    // with an existing blanket implementation.
    pub trait IntoPrimitiveId: Into<PrimitiveId> {}

    impl IntoPrimitiveId for JointId {}
    impl IntoPrimitiveId for SegId {}
    impl IntoPrimitiveId for ViaId {}
    impl IntoPrimitiveId for PolyId {}
}

impl<T: sealed::IntoPrimitiveId, U: sealed::IntoPrimitiveId> From<Infringement<T, U>>
    for Infringement<T, PrimitiveId>
where
    U: sealed::IntoPrimitiveId,
{
    fn from(from: Infringement<T, U>) -> Self {
        Infringement {
            infringer: from.infringer,
            infringee: from.infringee.into(),
        }
    }
}

impl<T: sealed::IntoPrimitiveId, U: sealed::IntoPrimitiveId> From<Infringement<T, U>>
    for Infringement
{
    fn from(from: Infringement<T, U>) -> Self {
        Infringement {
            infringer: from.infringer.into(),
            infringee: from.infringee.into(),
        }
    }
}

impl<T: sealed::IntoPrimitiveId> From<Infringement<T, PrimitiveId>> for Infringement {
    fn from(from: Infringement<T, PrimitiveId>) -> Self {
        Infringement {
            infringer: from.infringer.into(),
            infringee: from.infringee,
        }
    }
}

impl Layout {
    pub fn locate_component_infringements(
        &self,
        infringer: ComponentId,
    ) -> impl Iterator<Item = Infringement<ComponentId, ComponentId>> + '_ {
        let mut infringee_components = BTreeSet::new();

        for infringement in self.locate_component_primitive_infringements(infringer) {
            let Some(infringee_component) = self.primitive_component(infringement.infringee())
            else {
                continue;
            };

            if infringee_component == infringer {
                continue;
            }

            infringee_components.insert(infringee_component);
        }

        infringee_components
            .into_iter()
            .map(move |infringee| Infringement {
                infringer,
                infringee,
            })
    }

    pub fn locate_component_primitive_infringements(
        &self,
        infringer: ComponentId,
    ) -> impl Iterator<Item = Infringement> + '_ {
        let component = self.component(infringer);

        let joint_infringements = component
            .joints
            .iter()
            .copied()
            .flat_map(|joint_id| self.locate_joint_infringements(joint_id).map(Into::into));
        let seg_infringements = component.segs.iter().copied().flat_map(|seg_id| {
            self.locate_seg_infringements(seg_id)
                .map(Into::into)
        });
        let via_infringements = component
            .vias
            .iter()
            .copied()
            .flat_map(|via_id| self.locate_via_infringements(via_id).map(Into::into));
        let poly_infringements = component.polys.iter().copied().flat_map(|poly_id| {
            self.locate_poly_infringements(poly_id)
                .map(Into::into)
        });

        joint_infringements
            .chain(seg_infringements)
            .chain(via_infringements)
            .chain(poly_infringements)
    }

    pub fn locate_pin_infringements(
        &self,
        infringer: PinId,
    ) -> impl Iterator<Item = Infringement<PinId, PinId>> + '_ {
        let mut infringee_pins = BTreeSet::new();

        for infringement in self.locate_pin_primitive_infringements(infringer) {
            let Some(infringee_pin) = self.primitive_pin(infringement.infringee()) else {
                continue;
            };

            if infringee_pin == infringer {
                continue;
            }

            infringee_pins.insert(infringee_pin);
        }

        infringee_pins
            .into_iter()
            .map(move |infringee| Infringement {
                infringer,
                infringee,
            })
    }

    pub fn locate_pin_primitive_infringements(
        &self,
        infringer: PinId,
    ) -> impl Iterator<Item = Infringement> + '_ {
        let pin = self.pin(infringer);

        pin.joints
            .iter()
            .copied()
            .flat_map(|joint_id| self.locate_joint_infringements(joint_id).map(Into::into))
            .chain(pin.segs.iter().copied().flat_map(|seg_id| {
                self.locate_seg_infringements(seg_id)
                    .map(Into::into)
            }))
            .chain(
                pin.vias
                    .iter()
                    .copied()
                    .flat_map(|via_id| self.locate_via_infringements(via_id).map(Into::into)),
            )
            .chain(pin.polys.iter().copied().flat_map(|poly_id| {
                self.locate_poly_infringements(poly_id)
                    .map(Into::into)
            }))
    }

    pub fn locate_joint_infringements(
        &self,
        infringer: JointId,
    ) -> impl Iterator<Item = Infringement<JointId>> + '_ {
        self.locate_joint_joint_infringements(infringer)
            .map(Into::into)
            .chain(
                self.locate_joint_seg_infringements(infringer)
                    .map(Into::into),
            )
            .chain(
                self.locate_joint_via_infringements(infringer)
                    .map(Into::into),
            )
            .chain(
                self.locate_joint_poly_infringements(infringer)
                    .map(Into::into),
            )
    }

    pub fn locate_joint_joint_infringements(
        &self,
        infringer: JointId,
    ) -> impl Iterator<Item = Infringement<JointId, JointId>> + '_ {
        self.locate_same_infringements(infringer, self.joints_rtree().as_ref())
    }

    pub fn locate_joint_seg_infringements(
        &self,
        infringer: JointId,
    ) -> impl Iterator<Item = Infringement<JointId, SegId>> + '_ {
        self.locate_cross_infringements(infringer, self.segs_rtree().as_ref())
    }

    pub fn locate_joint_via_infringements(
        &self,
        infringer: JointId,
    ) -> impl Iterator<Item = Infringement<JointId, ViaId>> + '_ {
        self.locate_cross_infringements(infringer, self.vias_rtree().as_ref())
    }

    pub fn locate_joint_poly_infringements(
        &self,
        infringer: JointId,
    ) -> impl Iterator<Item = Infringement<JointId, PolyId>> + '_ {
        self.locate_cross_infringements(infringer, self.polys_rtree().as_ref())
    }

    pub fn locate_seg_infringements(
        &self,
        infringer: SegId,
    ) -> impl Iterator<Item = Infringement<SegId>> + '_ {
        self.locate_seg_joint_infringements(infringer)
            .map(Into::into)
            .chain(
                self.locate_seg_seg_infringements(infringer)
                    .map(Into::into),
            )
            .chain(
                self.locate_seg_via_infringements(infringer)
                    .map(Into::into),
            )
            .chain(
                self.locate_seg_poly_infringements(infringer)
                    .map(Into::into),
            )
    }

    pub fn locate_seg_joint_infringements(
        &self,
        infringer: SegId,
    ) -> impl Iterator<Item = Infringement<SegId, JointId>> + '_ {
        self.locate_cross_infringements(infringer, self.joints_rtree().as_ref())
    }

    pub fn locate_seg_seg_infringements(
        &self,
        infringer: SegId,
    ) -> impl Iterator<Item = Infringement<SegId, SegId>> + '_ {
        self.locate_same_infringements(infringer, self.segs_rtree().as_ref())
    }

    pub fn locate_seg_via_infringements(
        &self,
        infringer: SegId,
    ) -> impl Iterator<Item = Infringement<SegId, ViaId>> + '_ {
        self.locate_cross_infringements(infringer, self.vias_rtree().as_ref())
    }

    pub fn locate_seg_poly_infringements(
        &self,
        infringer: SegId,
    ) -> impl Iterator<Item = Infringement<SegId, PolyId>> + '_ {
        self.locate_cross_infringements(infringer, self.polys_rtree().as_ref())
    }

    pub fn locate_via_infringements(
        &self,
        infringer: ViaId,
    ) -> impl Iterator<Item = Infringement<ViaId>> + '_ {
        self.locate_via_joint_infringements(infringer)
            .map(Into::into)
            .chain(
                self.locate_via_seg_infringements(infringer)
                    .map(Into::into),
            )
            .chain(self.locate_via_via_infringements(infringer).map(Into::into))
            .chain(
                self.locate_via_poly_infringements(infringer)
                    .map(Into::into),
            )
    }

    pub fn locate_via_joint_infringements(
        &self,
        infringer: ViaId,
    ) -> impl Iterator<Item = Infringement<ViaId, JointId>> + '_ {
        self.locate_cross_infringements(infringer, self.joints_rtree().as_ref())
    }

    pub fn locate_via_seg_infringements(
        &self,
        infringer: ViaId,
    ) -> impl Iterator<Item = Infringement<ViaId, SegId>> + '_ {
        self.locate_cross_infringements(infringer, self.segs_rtree().as_ref())
    }

    pub fn locate_via_via_infringements(
        &self,
        infringer: ViaId,
    ) -> impl Iterator<Item = Infringement<ViaId, ViaId>> + '_ {
        self.locate_same_infringements(infringer, self.vias_rtree().as_ref())
    }

    pub fn locate_via_poly_infringements(
        &self,
        infringer: ViaId,
    ) -> impl Iterator<Item = Infringement<ViaId, PolyId>> + '_ {
        self.locate_cross_infringements(infringer, self.polys_rtree().as_ref())
    }

    pub fn locate_poly_infringements(
        &self,
        infringer: PolyId,
    ) -> impl Iterator<Item = Infringement<PolyId>> + '_ {
        self.locate_poly_joint_infringements(infringer)
            .map(Into::into)
            .chain(
                self.locate_poly_seg_infringements(infringer)
                    .map(Into::into),
            )
            .chain(
                self.locate_poly_via_infringements(infringer)
                    .map(Into::into),
            )
            .chain(
                self.locate_poly_poly_infringements(infringer)
                    .map(Into::into),
            )
    }

    pub fn locate_poly_joint_infringements(
        &self,
        infringer: PolyId,
    ) -> impl Iterator<Item = Infringement<PolyId, JointId>> + '_ {
        self.locate_cross_infringements(infringer, self.joints_rtree().as_ref())
    }

    pub fn locate_poly_seg_infringements(
        &self,
        infringer: PolyId,
    ) -> impl Iterator<Item = Infringement<PolyId, SegId>> + '_ {
        self.locate_cross_infringements(infringer, self.segs_rtree().as_ref())
    }

    pub fn locate_poly_via_infringements(
        &self,
        infringer: PolyId,
    ) -> impl Iterator<Item = Infringement<PolyId, ViaId>> + '_ {
        self.locate_cross_infringements(infringer, self.vias_rtree().as_ref())
    }

    pub fn locate_poly_poly_infringements(
        &self,
        infringer: PolyId,
    ) -> impl Iterator<Item = Infringement<PolyId, PolyId>> + '_ {
        self.locate_same_infringements(infringer, self.polys_rtree().as_ref())
    }

    fn locate_cross_infringements<
        'a,
        T: Copy + Into<PrimitiveId> + 'a,
        U: Copy + Into<PrimitiveId>,
    >(
        &'a self,
        infringer: T,
        infringee_tree: &'a RTree<GeomWithData<Rectangle<[i64; 3]>, U>>,
    ) -> impl Iterator<Item = Infringement<T, U>> + 'a {
        infringee_tree
            .locate_in_envelope_intersecting(&self.primitive_bbox_envelope(infringer.into()))
            .map(|infringee_geom| infringee_geom.data)
            .filter(move |&infringee| {
                !Self::nets_match(
                    self.primitive_net(infringer.into()),
                    self.primitive_net(infringee.into()),
                )
            })
            .map(move |infringee| Infringement {
                infringer,
                infringee,
            })
    }

    fn locate_same_infringements<'a, T: Copy + PartialEq + Into<PrimitiveId> + 'a>(
        &'a self,
        infringer: T,
        rtree: &'a RTree<GeomWithData<Rectangle<[i64; 3]>, T>>,
    ) -> impl Iterator<Item = Infringement<T, T>> + 'a {
        rtree
            .locate_in_envelope_intersecting(&self.primitive_bbox_envelope(infringer.into()))
            .map(|infringee_geom| infringee_geom.data)
            .filter(move |&infringee| infringee != infringer)
            .filter(move |&infringee| {
                !Self::nets_match(
                    self.primitive_net(infringer.into()),
                    self.primitive_net(infringee.into()),
                )
            })
            .map(move |infringee| Infringement {
                infringer,
                infringee,
            })
    }

    fn primitive_component(&self, primitive: PrimitiveId) -> Option<ComponentId> {
        match primitive {
            PrimitiveId::Joint(joint_id) => self.joint(joint_id).spec.component,
            PrimitiveId::Seg(seg_id) => self.seg(seg_id).spec.component,
            PrimitiveId::Via(via_id) => self.via(via_id).spec.component,
            PrimitiveId::Poly(poly_id) => self.poly(poly_id).spec.component,
        }
    }

    fn primitive_bbox_envelope(&self, primitive: PrimitiveId) -> AABB<[i64; 3]> {
        match primitive {
            PrimitiveId::Joint(joint_id) => self.joint(joint_id).bbox().aabb(),
            PrimitiveId::Seg(seg_id) => self.seg(seg_id).bbox().aabb(),
            PrimitiveId::Via(via_id) => self.via(via_id).bbox().aabb(),
            PrimitiveId::Poly(poly_id) => self.poly(poly_id).bbox().aabb(),
        }
    }

    pub fn primitive_net(&self, primitive: PrimitiveId) -> Option<NetId> {
        match primitive {
            PrimitiveId::Joint(joint_id) => self.joint(joint_id).spec.net,
            PrimitiveId::Seg(seg_id) => self.seg(seg_id).net,
            PrimitiveId::Via(via_id) => self.via(via_id).net,
            PrimitiveId::Poly(poly_id) => self.poly(poly_id).spec.net,
        }
    }

    fn nets_match(a: Option<NetId>, b: Option<NetId>) -> bool {
        matches!((a, b), (Some(net_a), Some(net_b)) if net_a == net_b)
    }
}
