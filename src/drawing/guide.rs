use enum_dispatch::enum_dispatch;
use geo::Line;

use crate::{
    drawing::{
        bend::BendIndex,
        dot::{DotIndex, FixedDotIndex, LooseDotIndex},
        graph::MakePrimitive,
        primitive::{GetCore, GetInnerOuter, GetOtherJoint, GetWeight, MakePrimitiveShape},
        rules::GetConditions,
        Drawing,
    },
    geometry::{
        primitive::{PrimitiveShape, PrimitiveShapeTrait},
        shape::ShapeTrait,
    },
    math::{self, Circle, NoTangents},
};

use super::{
    graph::PrimitiveIndex,
    primitive::GetJoints,
    rules::{Conditions, RulesTrait},
    segbend::Segbend,
};

#[enum_dispatch]
pub trait HeadTrait {
    fn face(&self) -> DotIndex;
}

#[enum_dispatch(HeadTrait)]
#[derive(Debug, Clone, Copy)]
pub enum Head {
    Bare(BareHead),
    Segbend(SegbendHead),
}

#[derive(Debug, Clone, Copy)]
pub struct BareHead {
    pub dot: FixedDotIndex,
}

impl HeadTrait for BareHead {
    fn face(&self) -> DotIndex {
        self.dot.into()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SegbendHead {
    pub face: LooseDotIndex,
    pub segbend: Segbend,
}

impl HeadTrait for SegbendHead {
    fn face(&self) -> DotIndex {
        self.face.into()
    }
}

pub struct Guide<'a, CW: Copy, R: RulesTrait> {
    drawing: &'a Drawing<CW, R>,
}

impl<'a, CW: Copy, R: RulesTrait> Guide<'a, CW, R> {
    pub fn new(drawing: &'a Drawing<CW, R>) -> Self {
        Self { drawing }
    }

    pub fn head_into_dot_segment(
        &self,
        head: &Head,
        into: FixedDotIndex,
        width: f64,
    ) -> Result<Line, NoTangents> {
        let from_circle = self.head_circle(head, width);
        let to_circle = Circle {
            pos: self.drawing.primitive(into).weight().circle.pos,
            r: 0.0,
        };

        let from_cw = self.head_cw(head);
        math::tangent_segment(from_circle, from_cw, to_circle, None)
    }

    pub fn head_around_dot_segments(
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

    pub fn head_around_dot_segment(
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

    pub fn head_around_dot_offset(&self, head: &Head, around: DotIndex, _width: f64) -> f64 {
        self.drawing.rules().clearance(
            &self.conditions(around.into()),
            &self.conditions(head.face().into()),
        )
    }

    pub fn head_around_bend_segments(
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

    pub fn head_around_bend_segment(
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

    pub fn head_around_bend_offset(&self, head: &Head, around: BendIndex, _width: f64) -> f64 {
        self.drawing.rules().clearance(
            &self.conditions(head.face().into()),
            &self.conditions(around.into()),
        )
    }

    pub fn head_cw(&self, head: &Head) -> Option<bool> {
        if let Head::Segbend(head) = head {
            let joints = self.drawing.primitive(head.segbend.bend).joints();

            if head.face() == joints.0.into() {
                Some(false)
            } else {
                Some(true)
            }
        } else {
            None
        }
    }

    fn head_circle(&self, head: &Head, width: f64) -> Circle {
        match *head {
            Head::Bare(head) => Circle {
                pos: head.face().primitive(self.drawing).shape().center(), // TODO.
                r: 0.0,
            },
            Head::Segbend(head) => {
                if let Some(inner) = self.drawing.primitive(head.segbend.bend).inner() {
                    self.bend_circle(inner.into(), width, &self.conditions(head.face().into()))
                } else {
                    self.dot_circle(
                        self.drawing.primitive(head.segbend.bend).core().into(),
                        width,
                        &self.conditions(head.face().into()),
                    )
                }
            }
        }
    }

    fn bend_circle(&self, bend: BendIndex, width: f64, guide_conditions: &Conditions) -> Circle {
        let outer_circle = match bend.primitive(self.drawing).shape() {
            PrimitiveShape::Bend(shape) => shape.outer_circle(),
            _ => unreachable!(),
        };

        Circle {
            pos: outer_circle.pos,
            r: outer_circle.r
                + width / 2.0
                + self
                    .drawing
                    .rules()
                    .clearance(&self.conditions(bend.into()), guide_conditions),
        }
    }

    fn dot_circle(&self, dot: DotIndex, width: f64, guide_conditions: &Conditions) -> Circle {
        let shape = dot.primitive(self.drawing).shape();
        Circle {
            pos: shape.center(),
            r: shape.width() / 2.0
                + width / 2.0
                + self
                    .drawing
                    .rules()
                    .clearance(&self.conditions(dot.into()), guide_conditions),
        }
    }

    pub fn segbend_head(&self, dot: LooseDotIndex) -> SegbendHead {
        SegbendHead {
            face: dot,
            segbend: self.drawing.segbend(dot),
        }
    }

    pub fn rear_head(&self, dot: LooseDotIndex) -> Head {
        self.head(self.rear(self.segbend_head(dot)))
    }

    pub fn head(&self, dot: DotIndex) -> Head {
        match dot {
            DotIndex::Fixed(fixed) => BareHead { dot: fixed }.into(),
            DotIndex::Loose(loose) => self.segbend_head(loose).into(),
        }
    }

    fn rear(&self, head: SegbendHead) -> DotIndex {
        self.drawing
            .primitive(head.segbend.seg)
            .other_joint(head.segbend.dot.into())
    }

    fn conditions(&self, node: PrimitiveIndex) -> Conditions {
        node.primitive(self.drawing).conditions()
    }
}
