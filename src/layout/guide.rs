use enum_dispatch::enum_dispatch;
use geo::Line;

use crate::{
    layout::{
        bend::BendIndex,
        connectivity::BandIndex,
        dot::{DotIndex, FixedDotIndex, LooseDotIndex},
        geometry::shape::{Shape, ShapeTrait},
        graph::{GetBandIndex, MakePrimitive},
        primitive::{GetCore, GetInnerOuter, GetOtherJoint, GetWeight, MakeShape},
        rules::GetConditions,
        Layout,
    },
    math::{self, Circle, NoTangents},
};

use super::{
    graph::GeometryIndex,
    primitive::GetJoints,
    rules::{Conditions, RulesTrait},
    segbend::Segbend,
};

#[enum_dispatch]
pub trait HeadTrait {
    fn face(&self) -> DotIndex;
    fn band(&self) -> BandIndex;
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
    pub band: BandIndex,
}

impl HeadTrait for BareHead {
    fn face(&self) -> DotIndex {
        self.dot.into()
    }

    fn band(&self) -> BandIndex {
        self.band
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SegbendHead {
    pub face: LooseDotIndex,
    pub segbend: Segbend,
    pub band: BandIndex,
}

impl HeadTrait for SegbendHead {
    fn face(&self) -> DotIndex {
        self.face.into()
    }

    fn band(&self) -> BandIndex {
        self.band
    }
}

pub struct Guide<'a, R: RulesTrait> {
    layout: &'a Layout<R>,
}

impl<'a, R: RulesTrait> Guide<'a, R> {
    pub fn new(layout: &'a Layout<R>) -> Self {
        Self { layout }
    }

    pub fn head_into_dot_segment(
        &self,
        head: &Head,
        into: FixedDotIndex,
        width: f64,
    ) -> Result<Line, NoTangents> {
        let from_circle = self.head_circle(head, width);
        let to_circle = Circle {
            pos: self.layout.primitive(into).weight().circle.pos,
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

    pub fn head_around_dot_offset(&self, head: &Head, around: DotIndex, width: f64) -> f64 {
        self.layout.rules().clearance(
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

    pub fn head_around_bend_offset(&self, head: &Head, around: BendIndex, width: f64) -> f64 {
        self.layout.rules().clearance(
            &self.conditions(head.face().into()),
            &self.conditions(around.into()),
        )
    }

    pub fn head_cw(&self, head: &Head) -> Option<bool> {
        if let Head::Segbend(head) = head {
            let joints = self.layout.primitive(head.segbend.bend).joints();

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
                pos: head.face().primitive(self.layout).shape().center(), // TODO.
                r: 0.0,
            },
            Head::Segbend(head) => {
                if let Some(inner) = self.layout.primitive(head.segbend.bend).inner() {
                    self.bend_circle(inner.into(), width, &self.conditions(head.face().into()))
                } else {
                    self.dot_circle(
                        self.layout.primitive(head.segbend.bend).core().into(),
                        width,
                        &self.conditions(head.face().into()),
                    )
                }
            }
        }
    }

    fn bend_circle(&self, bend: BendIndex, width: f64, guide_conditions: &Conditions) -> Circle {
        let outer_circle = match bend.primitive(self.layout).shape() {
            Shape::Bend(shape) => shape.outer_circle(),
            _ => unreachable!(),
        };

        Circle {
            pos: outer_circle.pos,
            r: outer_circle.r
                + width / 2.0
                + self
                    .layout
                    .rules()
                    .clearance(&self.conditions(bend.into()), guide_conditions),
        }
    }

    fn dot_circle(&self, dot: DotIndex, width: f64, guide_conditions: &Conditions) -> Circle {
        let shape = dot.primitive(self.layout).shape();
        Circle {
            pos: shape.center(),
            r: shape.width() / 2.0
                + width / 2.0
                + self
                    .layout
                    .rules()
                    .clearance(&self.conditions(dot.into()), guide_conditions),
        }
    }

    pub fn segbend_head(&self, dot: LooseDotIndex) -> SegbendHead {
        SegbendHead {
            face: dot,
            segbend: self.layout.segbend(dot),
            band: self.layout.primitive(dot).weight().band(),
        }
    }

    pub fn rear_head(&self, dot: LooseDotIndex) -> Head {
        self.head(
            self.rear(self.segbend_head(dot)),
            self.layout.primitive(dot).weight().band(),
        )
    }

    pub fn head(&self, dot: DotIndex, band: BandIndex) -> Head {
        match dot {
            DotIndex::Fixed(fixed) => BareHead { dot: fixed, band }.into(),
            DotIndex::Loose(loose) => self.segbend_head(loose).into(),
        }
    }

    fn rear(&self, head: SegbendHead) -> DotIndex {
        self.layout
            .primitive(head.segbend.seg)
            .other_joint(head.segbend.dot.into())
    }

    fn conditions(&self, node: GeometryIndex) -> Conditions {
        node.primitive(self.layout).conditions()
    }
}