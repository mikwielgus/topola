// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    compass::CompassDirection,
    layout::{
        Layout,
        compounds::{ComponentId, PinId},
        primitives::{JointId, PolygonId, PrimitiveId, SegmentId, ViaId},
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
            .copied()
            .flat_map(move |infringer_pin| {
                self.component(infringee)
                    .pins
                    .iter()
                    .copied()
                    .flat_map(move |infringee_pin| {
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
            PrimitiveId::Segment(infringer) => {
                self.segment_primitive_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Via(infringer) => {
                self.via_primitive_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Polygon(infringer) => {
                self.polygon_primitive_repulsion(infringer, infringee, orientation)
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

    pub fn joint_segment_repulsion(
        &self,
        infringer: JointId,
        infringee: SegmentId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.joint_segment_rect_overlap(infringer, infringee),
            self.joint(infringer).center(),
            self.segment(infringee).center(),
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

    pub fn joint_polygon_repulsion(
        &self,
        infringer: JointId,
        infringee: PolygonId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.joint_polygon_rect_overlap(infringer, infringee),
            self.joint(infringer).center(),
            self.polygon(infringee).center(),
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
            PrimitiveId::Segment(infringee) => {
                self.joint_segment_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Via(infringee) => {
                self.joint_via_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Polygon(infringee) => {
                self.joint_polygon_repulsion(infringer, infringee, orientation)
            }
        }
    }

    pub fn segment_joint_repulsion(
        &self,
        infringer: SegmentId,
        infringee: JointId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.segment_joint_rect_overlap(infringer, infringee),
            self.segment(infringer).center(),
            self.joint(infringee).center(),
            orientation,
        )
    }

    pub fn segment_segment_repulsion(
        &self,
        infringer: SegmentId,
        infringee: SegmentId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.segment_segment_rect_overlap(infringer, infringee),
            self.segment(infringer).center(),
            self.segment(infringee).center(),
            orientation,
        )
    }

    pub fn segment_via_repulsion(
        &self,
        infringer: SegmentId,
        infringee: ViaId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.segment_via_rect_overlap(infringer, infringee),
            self.segment(infringer).center(),
            self.via(infringee).position,
            orientation,
        )
    }

    pub fn segment_polygon_repulsion(
        &self,
        infringer: SegmentId,
        infringee: PolygonId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.segment_polygon_rect_overlap(infringer, infringee),
            self.segment(infringer).center(),
            self.polygon(infringee).center(),
            orientation,
        )
    }

    pub fn segment_primitive_repulsion(
        &self,
        infringer: SegmentId,
        infringee: PrimitiveId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        match infringee {
            PrimitiveId::Joint(infringee) => {
                self.segment_joint_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Segment(infringee) => {
                self.segment_segment_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Via(infringee) => {
                self.segment_via_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Polygon(infringee) => {
                self.segment_polygon_repulsion(infringer, infringee, orientation)
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

    pub fn via_segment_repulsion(
        &self,
        infringer: ViaId,
        infringee: SegmentId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.via_segment_rect_overlap(infringer, infringee),
            self.via(infringer).position,
            self.segment(infringee).center(),
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

    pub fn via_polygon_repulsion(
        &self,
        infringer: ViaId,
        infringee: PolygonId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.via_polygon_rect_overlap(infringer, infringee),
            self.via(infringer).position,
            self.polygon(infringee).center(),
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
            PrimitiveId::Segment(infringee) => {
                self.via_segment_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Via(infringee) => {
                self.via_via_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Polygon(infringee) => {
                self.via_polygon_repulsion(infringer, infringee, orientation)
            }
        }
    }

    pub fn polygon_joint_repulsion(
        &self,
        infringer: PolygonId,
        infringee: JointId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.polygon_joint_rect_overlap(infringer, infringee),
            self.polygon(infringer).center(),
            self.joint(infringee).center(),
            orientation,
        )
    }

    pub fn polygon_segment_repulsion(
        &self,
        infringer: PolygonId,
        infringee: SegmentId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.polygon_segment_rect_overlap(infringer, infringee),
            self.polygon(infringer).center(),
            self.segment(infringee).center(),
            orientation,
        )
    }

    pub fn polygon_via_repulsion(
        &self,
        infringer: PolygonId,
        infringee: ViaId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.polygon_via_rect_overlap(infringer, infringee),
            self.polygon(infringer).center(),
            self.via(infringee).position,
            orientation,
        )
    }

    pub fn polygon_polygon_repulsion(
        &self,
        infringer: PolygonId,
        infringee: PolygonId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        Self::repulsion_from_rect_overlap(
            self.polygon_polygon_rect_overlap(infringer, infringee),
            self.polygon(infringer).center(),
            self.polygon(infringee).center(),
            orientation,
        )
    }

    pub fn polygon_primitive_repulsion(
        &self,
        infringer: PolygonId,
        infringee: PrimitiveId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        match infringee {
            PrimitiveId::Joint(infringee) => {
                self.polygon_joint_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Segment(infringee) => {
                self.polygon_segment_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Via(infringee) => {
                self.polygon_via_repulsion(infringer, infringee, orientation)
            }
            PrimitiveId::Polygon(infringee) => {
                self.polygon_polygon_repulsion(infringer, infringee, orientation)
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
