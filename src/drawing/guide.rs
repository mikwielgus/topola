// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::Line;

use crate::{
    geometry::{primitive::PrimitiveShape, shape::AccessShape, GetWidth},
    math::{self, Circle, NoTangents, RotationSense},
};

use super::{
    bend::BendIndex,
    dot::{DotIndex, FixedDotIndex, LooseDotIndex},
    graph::{MakePrimitive, PrimitiveIndex},
    head::{BareHead, CaneHead, GetFace, Head},
    primitive::{GetCore, GetJoints, GetOtherJoint, GetWeight, MakePrimitiveShape},
    rules::{AccessRules, Conditions, GetConditions},
    Drawing,
};

impl<CW: Clone, Cel: Copy, R: AccessRules> Drawing<CW, Cel, R> {
    pub fn guide_for_head_into_dot_segment(
        &self,
        head: &Head,
        into: FixedDotIndex,
        width: f64,
    ) -> Result<Line, NoTangents> {
        let from_circle = self.head_circle(head, width);
        let to_circle = Circle {
            pos: self.primitive(into).weight().0.circle.pos,
            r: 0.0,
        };

        let from_sense = self.head_sense(head);
        math::tangent_segment(from_circle, from_sense, to_circle, None)
    }

    pub fn guide_for_head_around_dot_segments(
        &self,
        head: &Head,
        around: DotIndex,
        width: f64,
    ) -> Result<(Line, Line), NoTangents> {
        let from_circle = self.head_circle(head, width);
        let to_circle =
            self.dot_circle(around, width, self.conditions(head.face().into()).as_ref());

        let from_sense = self.head_sense(head);
        let tangents: Vec<Line> =
            math::tangent_segments(from_circle, from_sense, to_circle, None)?.collect();
        Ok((tangents[0], tangents[1]))
    }

    pub fn guide_for_head_around_dot_segment(
        &self,
        head: &Head,
        around: DotIndex,
        sense: RotationSense,
        width: f64,
    ) -> Result<Line, NoTangents> {
        let from_circle = self.head_circle(head, width);
        let to_circle =
            self.dot_circle(around, width, self.conditions(head.face().into()).as_ref());

        let from_sense = self.head_sense(head);
        math::tangent_segment(from_circle, from_sense, to_circle, Some(sense))
    }

    pub fn guide_for_head_around_dot_offset(
        &self,
        head: &Head,
        around: DotIndex,
        _width: f64,
    ) -> f64 {
        self.clearance(
            self.conditions(around.into()).as_ref(),
            self.conditions(head.face().into()).as_ref(),
        )
    }

    pub fn guide_for_head_around_bend_segments(
        &self,
        head: &Head,
        around: BendIndex,
        width: f64,
    ) -> Result<(Line, Line), NoTangents> {
        let from_circle = self.head_circle(head, width);
        let to_circle =
            self.bend_circle(around, width, self.conditions(head.face().into()).as_ref());

        let from_sense = self.head_sense(head);
        let tangents: Vec<Line> =
            math::tangent_segments(from_circle, from_sense, to_circle, None)?.collect();
        Ok((tangents[0], tangents[1]))
    }

    pub fn guide_for_head_around_bend_segment(
        &self,
        head: &Head,
        around: BendIndex,
        sense: RotationSense,
        width: f64,
    ) -> Result<Line, NoTangents> {
        let from_circle = self.head_circle(head, width);
        let to_circle =
            self.bend_circle(around, width, self.conditions(head.face().into()).as_ref());

        let from_sense = self.head_sense(head);
        math::tangent_segment(from_circle, from_sense, to_circle, Some(sense))
    }

    pub fn guide_for_head_around_bend_offset(
        &self,
        head: &Head,
        around: BendIndex,
        _width: f64,
    ) -> f64 {
        self.clearance(
            self.conditions(head.face().into()).as_ref(),
            self.conditions(around.into()).as_ref(),
        )
    }

    pub fn head_sense(&self, head: &Head) -> Option<RotationSense> {
        if let Head::Cane(head) = head {
            let joints = self.primitive(head.cane.bend).joints();

            if head.face() == joints.0.into() {
                Some(RotationSense::Counterclockwise)
            } else {
                Some(RotationSense::Clockwise)
            }
        } else {
            None
        }
    }

    pub fn cane_head(&self, face: LooseDotIndex) -> CaneHead {
        CaneHead {
            face,
            cane: self.cane(face),
        }
    }

    pub fn rear_head(&self, face: LooseDotIndex) -> Head {
        self.head(self.rear(self.cane_head(face)))
    }

    pub fn head(&self, face: DotIndex) -> Head {
        match face {
            DotIndex::Fixed(dot) => BareHead { face: dot }.into(),
            DotIndex::Loose(dot) => self.cane_head(dot).into(),
        }
    }

    pub fn dot_circle(
        &self,
        dot: DotIndex,
        width: f64,
        guide_conditions: Option<&Conditions<'_>>,
    ) -> Circle {
        let shape = dot.primitive(self).shape();
        Circle {
            pos: shape.center(),
            r: shape.width() / 2.0
                + width / 2.0
                + self.clearance(self.conditions(dot.into()).as_ref(), guide_conditions),
        }
    }

    pub fn bend_circle(
        &self,
        bend: BendIndex,
        width: f64,
        guide_conditions: Option<&Conditions<'_>>,
    ) -> Circle {
        let outer_circle = match bend.primitive(self).shape() {
            PrimitiveShape::Bend(shape) => shape.outer_circle(),
            _ => unreachable!(),
        };

        Circle {
            pos: outer_circle.pos,
            r: outer_circle.r
                + width / 2.0
                + self.clearance(self.conditions(bend.into()).as_ref(), guide_conditions),
        }
    }

    pub fn conditions(&self, node: PrimitiveIndex) -> Option<Conditions<'_>> {
        node.primitive(self).conditions()
    }

    fn clearance(&self, lhs: Option<&Conditions<'_>>, rhs: Option<&Conditions<'_>>) -> f64 {
        match (lhs, rhs) {
            (None, _) | (_, None) => 0.0,
            (Some(lhs), Some(rhs)) => self.rules().clearance(lhs, rhs),
        }
    }

    fn head_circle(&self, head: &Head, width: f64) -> Circle {
        match *head {
            Head::Bare(head) => Circle {
                pos: head.face().primitive(self).shape().center(), // TODO.
                r: 0.0,
            },
            Head::Cane(head) => {
                if let Some(inner) = self.primitive(head.cane.bend).inner() {
                    self.bend_circle(
                        inner.into(),
                        width,
                        self.conditions(head.face().into()).as_ref(),
                    )
                } else {
                    self.dot_circle(
                        self.primitive(head.cane.bend).core().into(),
                        width,
                        self.conditions(head.face().into()).as_ref(),
                    )
                }
            }
        }
    }

    fn rear(&self, head: CaneHead) -> DotIndex {
        self.primitive(head.cane.seg)
            .other_joint(head.cane.dot.into())
    }
}
