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
use super::primitives::{JointId, PolygonId, SegmentId, ViaId};

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
    impl IntoPrimitiveId for SegmentId {}
    impl IntoPrimitiveId for ViaId {}
    impl IntoPrimitiveId for PolygonId {}
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
        let segment_infringements = component.segments.iter().copied().flat_map(|segment_id| {
            self.locate_segment_infringements(segment_id)
                .map(Into::into)
        });
        let via_infringements = component
            .vias
            .iter()
            .copied()
            .flat_map(|via_id| self.locate_via_infringements(via_id).map(Into::into));
        let polygon_infringements = component.polygons.iter().copied().flat_map(|polygon_id| {
            self.locate_polygon_infringements(polygon_id)
                .map(Into::into)
        });

        joint_infringements
            .chain(segment_infringements)
            .chain(via_infringements)
            .chain(polygon_infringements)
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
            .chain(
                pin.segments.iter().copied().flat_map(|segment_id| {
                    self.locate_segment_infringements(segment_id)
                        .map(Into::into)
                }),
            )
            .chain(
                pin.vias
                    .iter()
                    .copied()
                    .flat_map(|via_id| self.locate_via_infringements(via_id).map(Into::into)),
            )
            .chain(pin.polygons.iter().copied().flat_map(|polygon_id| {
                self.locate_polygon_infringements(polygon_id)
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
                self.locate_joint_segment_infringements(infringer)
                    .map(Into::into),
            )
            .chain(
                self.locate_joint_via_infringements(infringer)
                    .map(Into::into),
            )
            .chain(
                self.locate_joint_polygon_infringements(infringer)
                    .map(Into::into),
            )
    }

    pub fn locate_joint_joint_infringements(
        &self,
        infringer: JointId,
    ) -> impl Iterator<Item = Infringement<JointId, JointId>> + '_ {
        self.locate_same_infringements(infringer, self.joints_rtree().as_ref())
    }

    pub fn locate_joint_segment_infringements(
        &self,
        infringer: JointId,
    ) -> impl Iterator<Item = Infringement<JointId, SegmentId>> + '_ {
        self.locate_cross_infringements(infringer, self.segments_rtree().as_ref())
    }

    pub fn locate_joint_via_infringements(
        &self,
        infringer: JointId,
    ) -> impl Iterator<Item = Infringement<JointId, ViaId>> + '_ {
        self.locate_cross_infringements(infringer, self.vias_rtree().as_ref())
    }

    pub fn locate_joint_polygon_infringements(
        &self,
        infringer: JointId,
    ) -> impl Iterator<Item = Infringement<JointId, PolygonId>> + '_ {
        self.locate_cross_infringements(infringer, self.polygons_rtree().as_ref())
    }

    pub fn locate_segment_infringements(
        &self,
        infringer: SegmentId,
    ) -> impl Iterator<Item = Infringement<SegmentId>> + '_ {
        self.locate_segment_joint_infringements(infringer)
            .map(Into::into)
            .chain(
                self.locate_segment_segment_infringements(infringer)
                    .map(Into::into),
            )
            .chain(
                self.locate_segment_via_infringements(infringer)
                    .map(Into::into),
            )
            .chain(
                self.locate_segment_polygon_infringements(infringer)
                    .map(Into::into),
            )
    }

    pub fn locate_segment_joint_infringements(
        &self,
        infringer: SegmentId,
    ) -> impl Iterator<Item = Infringement<SegmentId, JointId>> + '_ {
        self.locate_cross_infringements(infringer, self.joints_rtree().as_ref())
    }

    pub fn locate_segment_segment_infringements(
        &self,
        infringer: SegmentId,
    ) -> impl Iterator<Item = Infringement<SegmentId, SegmentId>> + '_ {
        self.locate_same_infringements(infringer, self.segments_rtree().as_ref())
    }

    pub fn locate_segment_via_infringements(
        &self,
        infringer: SegmentId,
    ) -> impl Iterator<Item = Infringement<SegmentId, ViaId>> + '_ {
        self.locate_cross_infringements(infringer, self.vias_rtree().as_ref())
    }

    pub fn locate_segment_polygon_infringements(
        &self,
        infringer: SegmentId,
    ) -> impl Iterator<Item = Infringement<SegmentId, PolygonId>> + '_ {
        self.locate_cross_infringements(infringer, self.polygons_rtree().as_ref())
    }

    pub fn locate_via_infringements(
        &self,
        infringer: ViaId,
    ) -> impl Iterator<Item = Infringement<ViaId>> + '_ {
        self.locate_via_joint_infringements(infringer)
            .map(Into::into)
            .chain(
                self.locate_via_segment_infringements(infringer)
                    .map(Into::into),
            )
            .chain(self.locate_via_via_infringements(infringer).map(Into::into))
            .chain(
                self.locate_via_polygon_infringements(infringer)
                    .map(Into::into),
            )
    }

    pub fn locate_via_joint_infringements(
        &self,
        infringer: ViaId,
    ) -> impl Iterator<Item = Infringement<ViaId, JointId>> + '_ {
        self.locate_cross_infringements(infringer, self.joints_rtree().as_ref())
    }

    pub fn locate_via_segment_infringements(
        &self,
        infringer: ViaId,
    ) -> impl Iterator<Item = Infringement<ViaId, SegmentId>> + '_ {
        self.locate_cross_infringements(infringer, self.segments_rtree().as_ref())
    }

    pub fn locate_via_via_infringements(
        &self,
        infringer: ViaId,
    ) -> impl Iterator<Item = Infringement<ViaId, ViaId>> + '_ {
        self.locate_same_infringements(infringer, self.vias_rtree().as_ref())
    }

    pub fn locate_via_polygon_infringements(
        &self,
        infringer: ViaId,
    ) -> impl Iterator<Item = Infringement<ViaId, PolygonId>> + '_ {
        self.locate_cross_infringements(infringer, self.polygons_rtree().as_ref())
    }

    pub fn locate_polygon_infringements(
        &self,
        infringer: PolygonId,
    ) -> impl Iterator<Item = Infringement<PolygonId>> + '_ {
        self.locate_polygon_joint_infringements(infringer)
            .map(Into::into)
            .chain(
                self.locate_polygon_segment_infringements(infringer)
                    .map(Into::into),
            )
            .chain(
                self.locate_polygon_via_infringements(infringer)
                    .map(Into::into),
            )
            .chain(
                self.locate_polygon_polygon_infringements(infringer)
                    .map(Into::into),
            )
    }

    pub fn locate_polygon_joint_infringements(
        &self,
        infringer: PolygonId,
    ) -> impl Iterator<Item = Infringement<PolygonId, JointId>> + '_ {
        self.locate_cross_infringements(infringer, self.joints_rtree().as_ref())
    }

    pub fn locate_polygon_segment_infringements(
        &self,
        infringer: PolygonId,
    ) -> impl Iterator<Item = Infringement<PolygonId, SegmentId>> + '_ {
        self.locate_cross_infringements(infringer, self.segments_rtree().as_ref())
    }

    pub fn locate_polygon_via_infringements(
        &self,
        infringer: PolygonId,
    ) -> impl Iterator<Item = Infringement<PolygonId, ViaId>> + '_ {
        self.locate_cross_infringements(infringer, self.vias_rtree().as_ref())
    }

    pub fn locate_polygon_polygon_infringements(
        &self,
        infringer: PolygonId,
    ) -> impl Iterator<Item = Infringement<PolygonId, PolygonId>> + '_ {
        self.locate_same_infringements(infringer, self.polygons_rtree().as_ref())
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
            PrimitiveId::Segment(segment_id) => self.segment(segment_id).spec.component,
            PrimitiveId::Via(via_id) => self.via(via_id).spec.component,
            PrimitiveId::Polygon(polygon_id) => self.polygon(polygon_id).component,
        }
    }

    fn primitive_bbox_envelope(&self, primitive: PrimitiveId) -> AABB<[i64; 3]> {
        match primitive {
            PrimitiveId::Joint(joint_id) => self.joint(joint_id).bbox().aabb(),
            PrimitiveId::Segment(segment_id) => self.segment(segment_id).bbox().aabb(),
            PrimitiveId::Via(via_id) => self.via(via_id).bbox().aabb(),
            PrimitiveId::Polygon(polygon_id) => self.polygon(polygon_id).bbox().aabb(),
        }
    }

    pub fn primitive_net(&self, primitive: PrimitiveId) -> Option<NetId> {
        match primitive {
            PrimitiveId::Joint(joint_id) => self.joint(joint_id).spec.net,
            PrimitiveId::Segment(segment_id) => self.segment(segment_id).net,
            PrimitiveId::Via(via_id) => self.via(via_id).net,
            PrimitiveId::Polygon(polygon_id) => self.polygon(polygon_id).net,
        }
    }

    fn nets_match(a: Option<NetId>, b: Option<NetId>) -> bool {
        matches!((a, b), (Some(net_a), Some(net_b)) if net_a == net_b)
    }
}
