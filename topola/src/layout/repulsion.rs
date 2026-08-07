// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    compass::CompassDirection,
    layout::{
        Layout,
        compounds::{ComponentId, PinId},
        primitives::{JointId, PolyId, PrimitiveId, SegId, ViaId},
    },
    orientation::Orientation,
    rect::Rect2,
    vector::Vector2,
};

impl Layout {
    pub fn locate_component_repulsions(
        &self,
        infringer: ComponentId,
        orientation: Orientation,
    ) -> impl Iterator<Item = Vector2<i64>> + '_ {
        self.locate_component_infringements(infringer)
            .flat_map(move |infringement| {
                self.component_component_repulsions(
                    infringement.infringer(),
                    infringement.infringee(),
                    orientation,
                )
            })
    }

    pub fn component_component_repulsions(
        &self,
        infringer: ComponentId,
        infringee: ComponentId,
        orientation: Orientation,
    ) -> impl Iterator<Item = Vector2<i64>> + '_ {
        self.component(infringer)
            .pins
            .iter()
            .flat_map(move |&infringer_pin| {
                self.component(infringee)
                    .pins
                    .iter()
                    .flat_map(move |&infringee_pin| {
                        self.pin_pin_repulsions(infringer_pin, infringee_pin, orientation)
                    })
            })
    }

    pub fn locate_pin_repulsions(
        &self,
        infringer: PinId,
        orientation: Orientation,
    ) -> impl Iterator<Item = Vector2<i64>> + '_ {
        self.locate_pin_infringements(infringer)
            .flat_map(move |infringement| {
                self.pin_pin_repulsions(
                    infringement.infringer(),
                    infringement.infringee(),
                    orientation,
                )
            })
    }

    pub fn pin_pin_repulsions(
        &self,
        infringer: PinId,
        infringee: PinId,
        orientation: Orientation,
    ) -> impl Iterator<Item = Vector2<i64>> + '_ {
        self.pin(infringer)
            .primitives()
            .flat_map(move |infringer_primitive| {
                self.pin(infringee)
                    .primitives()
                    .map(move |infringee_primitive| {
                        self.primitive_primitive_repulsion(
                            infringer_primitive,
                            infringee_primitive,
                            orientation,
                        )
                    })
            })
    }

    pub fn primitive_primitive_repulsion(
        &self,
        infringer: PrimitiveId,
        infringee: PrimitiveId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        match infringer {
            PrimitiveId::Joint(infringer) => {
                self.joint_primitive_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Seg(infringer) => {
                self.seg_primitive_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Via(infringer) => {
                self.via_primitive_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Poly(infringer) => {
                self.poly_primitive_repulsion(infringer, infringee, orientation)
            }
        }
    }

    pub fn joint_joint_repulsion(
        &self,
        infringer: JointId,
        infringee: JointId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.joint_joint_rect_overlap(infringer, infringee),
            self.joint(infringer).center(),
            self.joint(infringee).center(),
            orientation,
        )
    }

    pub fn joint_seg_repulsion(
        &self,
        infringer: JointId,
        infringee: SegId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.joint_seg_rect_overlap(infringer, infringee),
            self.joint(infringer).center(),
            self.seg(infringee).center(),
            orientation,
        )
    }

    pub fn joint_via_repulsion(
        &self,
        infringer: JointId,
        infringee: ViaId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.joint_via_rect_overlap(infringer, infringee),
            self.joint(infringer).center(),
            self.via(infringee).position,
            orientation,
        )
    }

    pub fn joint_poly_repulsion(
        &self,
        infringer: JointId,
        infringee: PolyId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.joint_poly_rect_overlap(infringer, infringee),
            self.joint(infringer).center(),
            self.poly(infringee).centroid,
            orientation,
        )
    }

    pub fn joint_primitive_repulsion(
        &self,
        infringer: JointId,
        infringee: PrimitiveId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        match infringee {
            PrimitiveId::Joint(infringee) => {
                self.joint_joint_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Seg(infringee) => {
                self.joint_seg_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Via(infringee) => {
                self.joint_via_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Poly(infringee) => {
                self.joint_poly_repulsion(infringer, infringee, orientation)
            }
        }
    }

    pub fn seg_joint_repulsion(
        &self,
        infringer: SegId,
        infringee: JointId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.seg_joint_rect_overlap(infringer, infringee),
            self.seg(infringer).center(),
            self.joint(infringee).center(),
            orientation,
        )
    }

    pub fn seg_seg_repulsion(
        &self,
        infringer: SegId,
        infringee: SegId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.seg_seg_rect_overlap(infringer, infringee),
            self.seg(infringer).center(),
            self.seg(infringee).center(),
            orientation,
        )
    }

    pub fn seg_via_repulsion(
        &self,
        infringer: SegId,
        infringee: ViaId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.seg_via_rect_overlap(infringer, infringee),
            self.seg(infringer).center(),
            self.via(infringee).position,
            orientation,
        )
    }

    pub fn seg_poly_repulsion(
        &self,
        infringer: SegId,
        infringee: PolyId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.seg_poly_rect_overlap(infringer, infringee),
            self.seg(infringer).center(),
            self.poly(infringee).centroid,
            orientation,
        )
    }

    pub fn seg_primitive_repulsion(
        &self,
        infringer: SegId,
        infringee: PrimitiveId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        match infringee {
            PrimitiveId::Joint(infringee) => {
                self.seg_joint_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Seg(infringee) => {
                self.seg_seg_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Via(infringee) => {
                self.seg_via_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Poly(infringee) => {
                self.seg_poly_repulsion(infringer, infringee, orientation)
            }
        }
    }

    pub fn via_joint_repulsion(
        &self,
        infringer: ViaId,
        infringee: JointId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.via_joint_rect_overlap(infringer, infringee),
            self.via(infringer).position,
            self.joint(infringee).center(),
            orientation,
        )
    }

    pub fn via_seg_repulsion(
        &self,
        infringer: ViaId,
        infringee: SegId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.via_seg_rect_overlap(infringer, infringee),
            self.via(infringer).position,
            self.seg(infringee).center(),
            orientation,
        )
    }

    pub fn via_via_repulsion(
        &self,
        infringer: ViaId,
        infringee: ViaId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.via_via_rect_overlap(infringer, infringee),
            self.via(infringer).position,
            self.via(infringee).position,
            orientation,
        )
    }

    pub fn via_poly_repulsion(
        &self,
        infringer: ViaId,
        infringee: PolyId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.via_poly_rect_overlap(infringer, infringee),
            self.via(infringer).position,
            self.poly(infringee).centroid,
            orientation,
        )
    }

    pub fn via_primitive_repulsion(
        &self,
        infringer: ViaId,
        infringee: PrimitiveId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        match infringee {
            PrimitiveId::Joint(infringee) => {
                self.via_joint_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Seg(infringee) => {
                self.via_seg_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Via(infringee) => {
                self.via_via_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Poly(infringee) => {
                self.via_poly_repulsion(infringer, infringee, orientation)
            }
        }
    }

    pub fn poly_joint_repulsion(
        &self,
        infringer: PolyId,
        infringee: JointId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.poly_joint_rect_overlap(infringer, infringee),
            self.poly(infringer).centroid,
            self.joint(infringee).center(),
            orientation,
        )
    }

    pub fn poly_seg_repulsion(
        &self,
        infringer: PolyId,
        infringee: SegId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.poly_seg_rect_overlap(infringer, infringee),
            self.poly(infringer).centroid,
            self.seg(infringee).center(),
            orientation,
        )
    }

    pub fn poly_via_repulsion(
        &self,
        infringer: PolyId,
        infringee: ViaId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.poly_via_rect_overlap(infringer, infringee),
            self.poly(infringer).centroid,
            self.via(infringee).position,
            orientation,
        )
    }

    pub fn poly_poly_repulsion(
        &self,
        infringer: PolyId,
        infringee: PolyId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.poly_poly_rect_overlap(infringer, infringee),
            self.poly(infringer).centroid,
            self.poly(infringee).centroid,
            orientation,
        )
    }

    pub fn poly_primitive_repulsion(
        &self,
        infringer: PolyId,
        infringee: PrimitiveId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        match infringee {
            PrimitiveId::Joint(infringee) => {
                self.poly_joint_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Seg(infringee) => {
                self.poly_seg_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Via(infringee) => {
                self.poly_via_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Poly(infringee) => {
                self.poly_poly_repulsion(infringer, infringee, orientation)
            }
        }
    }

    fn repulsion_from_rect_overlap(
        overlap: Option<Rect2<i64>>,
        infringer_pos: Vector2<i64>,
        infringee_pos: Vector2<i64>,
        orientation: Orientation,
    ) -> Vector2<i64> {
        let Some(overlap) = overlap else {
            return Vector2::new(0, 0);
        };

        let Some(wind) = orientation.principal_wind(infringer_pos - infringee_pos) else {
            return Vector2::new(0, 0);
        };

        wind.cast_vector(overlap.size())
    }
}
