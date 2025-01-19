// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::Line;

use crate::{
    geometry::{
        primitive::{AccessPrimitiveShape, PrimitiveShape},
        shape::AccessShape,
    },
    math::{self, Circle, NoTangents},
};

use super::{
    bend::BendIndex,
    dot::{DotIndex, FixedDotIndex, LooseDotIndex},
    graph::{MakePrimitive, PrimitiveIndex},
    head::{BareHead, CaneHead, GetFace, Head},
    primitive::{GetCore, GetInnerOuter, GetJoints, GetOtherJoint, GetWeight, MakePrimitiveShape},
    rules::{AccessRules, Conditions, GetConditions},
    Drawing,
};

pub trait Guide {
    fn head_into_dot_segment(
        &self,
        head: &Head,
        into: FixedDotIndex,
        width: f64,
    ) -> Result<Line, NoTangents>;

    fn head_around_dot_segments(
        &self,
        head: &Head,
        around: DotIndex,
        width: f64,
    ) -> Result<(Line, Line), NoTangents>;

    fn head_around_dot_segment(
        &self,
        head: &Head,
        around: DotIndex,
        cw: bool,
        width: f64,
    ) -> Result<Line, NoTangents>;

    fn head_around_dot_offset(&self, head: &Head, around: DotIndex, _width: f64) -> f64;

    fn head_around_bend_segments(
        &self,
        head: &Head,
        around: BendIndex,
        width: f64,
    ) -> Result<(Line, Line), NoTangents>;

    fn head_around_bend_segment(
        &self,
        head: &Head,
        around: BendIndex,
        cw: bool,
        width: f64,
    ) -> Result<Line, NoTangents>;

    fn head_around_bend_offset(&self, head: &Head, around: BendIndex, _width: f64) -> f64;

    fn head_cw(&self, head: &Head) -> Option<bool>;

    fn cane_head(&self, face: LooseDotIndex) -> CaneHead;

    fn rear_head(&self, face: LooseDotIndex) -> Head;

    fn head(&self, face: DotIndex) -> Head;
}

impl<CW: Copy, R: AccessRules> Guide for Drawing<CW, R> {
    fn head_into_dot_segment(
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

        let from_cw = self.head_cw(head);
        math::tangent_segment(from_circle, from_cw, to_circle, None)
    }

    fn head_around_dot_segments(
        &self,
        head: &Head,
        around: DotIndex,
        width: f64,
    ) -> Result<(Line, Line), NoTangents> {
        let from_circle = self.head_circle(head, width);
        let to_circle = self.dot_circle(around, width, &self.conditions(head.face().into()));

        let from_cw = self.head_cw(head);
        let tangents: Vec<Line> =
            math::tangent_segments(from_circle, from_cw, to_circle, None)?.collect();
        Ok((tangents[0], tangents[1]))
    }

    fn head_around_dot_segment(
        &self,
        head: &Head,
        around: DotIndex,
        cw: bool,
        width: f64,
    ) -> Result<Line, NoTangents> {
        let from_circle = self.head_circle(head, width);
        let to_circle = self.dot_circle(around, width, &self.conditions(head.face().into()));

        let from_cw = self.head_cw(head);
        math::tangent_segment(from_circle, from_cw, to_circle, Some(cw))
    }

    fn head_around_dot_offset(&self, head: &Head, around: DotIndex, _width: f64) -> f64 {
        self.rules().clearance(
            &self.conditions(around.into()),
            &self.conditions(head.face().into()),
        )
    }

    fn head_around_bend_segments(
        &self,
        head: &Head,
        around: BendIndex,
        width: f64,
    ) -> Result<(Line, Line), NoTangents> {
        let from_circle = self.head_circle(head, width);
        let to_circle = self.bend_circle(around, width, &self.conditions(head.face().into()));

        let from_cw = self.head_cw(head);
        let tangents: Vec<Line> =
            math::tangent_segments(from_circle, from_cw, to_circle, None)?.collect();
        Ok((tangents[0], tangents[1]))
    }

    fn head_around_bend_segment(
        &self,
        head: &Head,
        around: BendIndex,
        cw: bool,
        width: f64,
    ) -> Result<Line, NoTangents> {
        let from_circle = self.head_circle(head, width);
        let to_circle = self.bend_circle(around, width, &self.conditions(head.face().into()));

        let from_cw = self.head_cw(head);
        math::tangent_segment(from_circle, from_cw, to_circle, Some(cw))
    }

    fn head_around_bend_offset(&self, head: &Head, around: BendIndex, _width: f64) -> f64 {
        self.rules().clearance(
            &self.conditions(head.face().into()),
            &self.conditions(around.into()),
        )
    }

    fn head_cw(&self, head: &Head) -> Option<bool> {
        if let Head::Cane(head) = head {
            let joints = self.primitive(head.cane.bend).joints();

            if head.face() == joints.0.into() {
                Some(false)
            } else {
                Some(true)
            }
        } else {
            None
        }
    }

    fn cane_head(&self, face: LooseDotIndex) -> CaneHead {
        CaneHead {
            face,
            cane: self.cane(face),
        }
    }

    fn rear_head(&self, face: LooseDotIndex) -> Head {
        self.head(self.rear(self.cane_head(face)))
    }

    fn head(&self, face: DotIndex) -> Head {
        match face {
            DotIndex::Fixed(dot) => BareHead { face: dot }.into(),
            DotIndex::Loose(dot) => self.cane_head(dot).into(),
        }
    }
}

trait GuidePrivate {
    fn head_circle(&self, head: &Head, width: f64) -> Circle;

    fn bend_circle(&self, bend: BendIndex, width: f64, guide_conditions: &Conditions) -> Circle;

    fn dot_circle(&self, dot: DotIndex, width: f64, guide_conditions: &Conditions) -> Circle;

    fn rear(&self, head: CaneHead) -> DotIndex;

    fn conditions(&self, node: PrimitiveIndex) -> Conditions;
}

impl<CW: Copy, R: AccessRules> GuidePrivate for Drawing<CW, R> {
    fn head_circle(&self, head: &Head, width: f64) -> Circle {
        match *head {
            Head::Bare(head) => Circle {
                pos: head.face().primitive(self).shape().center(), // TODO.
                r: 0.0,
            },
            Head::Cane(head) => {
                if let Some(inner) = self.primitive(head.cane.bend).inner() {
                    self.bend_circle(inner.into(), width, &self.conditions(head.face().into()))
                } else {
                    self.dot_circle(
                        self.primitive(head.cane.bend).core().into(),
                        width,
                        &self.conditions(head.face().into()),
                    )
                }
            }
        }
    }

    fn bend_circle(&self, bend: BendIndex, width: f64, guide_conditions: &Conditions) -> Circle {
        let outer_circle = match bend.primitive(self).shape() {
            PrimitiveShape::Bend(shape) => shape.outer_circle(),
            _ => unreachable!(),
        };

        Circle {
            pos: outer_circle.pos,
            r: outer_circle.r
                + width / 2.0
                + self
                    .rules()
                    .clearance(&self.conditions(bend.into()), guide_conditions),
        }
    }

    fn dot_circle(&self, dot: DotIndex, width: f64, guide_conditions: &Conditions) -> Circle {
        let shape = dot.primitive(self).shape();
        Circle {
            pos: shape.center(),
            r: shape.width() / 2.0
                + width / 2.0
                + self
                    .rules()
                    .clearance(&self.conditions(dot.into()), guide_conditions),
        }
    }

    fn rear(&self, head: CaneHead) -> DotIndex {
        self.primitive(head.cane.seg)
            .other_joint(head.cane.dot.into())
    }

    fn conditions(&self, node: PrimitiveIndex) -> Conditions {
        node.primitive(self).conditions()
    }
}
