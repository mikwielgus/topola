// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    Board, Rect2, Vector2,
    compass::CompassDirection,
    layout::compounds::ComponentId,
    orientation::Orientation,
    primitives::{JointId, PolygonId, PrimitiveId, SegmentId, ViaId},
};

impl Board {
    pub fn component_component_repulsion(
        &self,
        infringer: ComponentId,
        infringee: ComponentId,
        orientation: Orientation,
    ) -> Vector2<i64> {
        let mut max_repulsion = Vector2::new(0, 0);
        let mut max_repulsion_magnitude = 0;

        for infringer_primitive in self.layout().component(infringer).primitives() {
            for infringee_primitive in self.layout().component(infringee).primitives() {
                let repulsion = self.primitive_primitive_repulsion(
                    infringer_primitive,
                    infringee_primitive,
                    orientation,
                );
                let repulsion_magnitude = repulsion.x.abs() + repulsion.y.abs();

                if repulsion_magnitude > max_repulsion_magnitude {
                    max_repulsion = repulsion;
                    max_repulsion_magnitude = repulsion_magnitude;
                }
            }
        }

        max_repulsion
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
            self.layout().joint(infringer).center(),
            self.layout().joint(infringee).center(),
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
            self.layout().joint(infringer).center(),
            self.layout().segment(infringee).center(),
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
            self.layout().joint(infringer).center(),
            self.layout().via(infringee).position,
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
            self.layout().joint(infringer).center(),
            self.layout().polygon(infringee).center(),
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
            self.layout().segment(infringer).center(),
            self.layout().joint(infringee).center(),
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
            self.layout().segment(infringer).center(),
            self.layout().segment(infringee).center(),
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
            self.layout().segment(infringer).center(),
            self.layout().via(infringee).position,
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
            self.layout().segment(infringer).center(),
            self.layout().polygon(infringee).center(),
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
            self.layout().via(infringer).position,
            self.layout().joint(infringee).center(),
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
            self.layout().via(infringer).position,
            self.layout().segment(infringee).center(),
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
            self.layout().via(infringer).position,
            self.layout().via(infringee).position,
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
            self.layout().via(infringer).position,
            self.layout().polygon(infringee).center(),
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
            self.layout().polygon(infringer).center(),
            self.layout().joint(infringee).center(),
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
            self.layout().polygon(infringer).center(),
            self.layout().segment(infringee).center(),
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
            self.layout().polygon(infringer).center(),
            self.layout().via(infringee).position,
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
            self.layout().polygon(infringer).center(),
            self.layout().polygon(infringee).center(),
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
