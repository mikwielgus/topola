use enum_dispatch::enum_dispatch;
use geo::Line;

use crate::{
    connectivity::BandIndex,
    geometry::{BendIndex, DotIndex, FixedDotIndex, GetBandIndex, LooseDotIndex, MakePrimitive},
    layout::Layout,
    math::{self, Circle, NoTangents},
    primitive::{GetCore, GetInnerOuter, GetOtherEnd, GetWeight, MakeShape},
    rules::{Conditions, Rules},
    segbend::Segbend,
    shape::{Shape, ShapeTrait},
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

pub struct Guide<'a, 'b> {
    layout: &'a Layout,
    rules: &'a Rules,
    conditions: &'b Conditions,
}

impl<'a, 'b> Guide<'a, 'b> {
    pub fn new(layout: &'a Layout, rules: &'a Rules, conditions: &'b Conditions) -> Self {
        Self {
            layout,
            rules,
            conditions,
        }
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
        let to_circle = self.dot_circle(around, width);

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
        let to_circle = self.dot_circle(around, width);

        let from_cw = self.head_cw(head);
        math::tangent_segment(from_circle, from_cw, to_circle, Some(cw))
    }

    pub fn head_around_bend_segments(
        &self,
        head: &Head,
        around: BendIndex,
        width: f64,
    ) -> Result<(Line, Line), NoTangents> {
        let from_circle = self.head_circle(head, width);
        let to_circle = self.bend_circle(around, width);

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
        let to_circle = self.bend_circle(around, width);

        let from_cw = self.head_cw(head);
        math::tangent_segment(from_circle, from_cw, to_circle, Some(cw))
    }

    pub fn head_cw(&self, head: &Head) -> Option<bool> {
        if let Head::Segbend(head) = head {
            Some(self.layout.primitive(head.segbend.bend).weight().cw)
        } else {
            None
        }
    }

    fn head_circle(&self, head: &Head, width: f64) -> Circle {
        let _conditions = Conditions {
            lower_net: None,
            higher_net: None,
            layer: None,
            zone: None,
        };

        match *head {
            Head::Bare(head) => Circle {
                pos: head.face().primitive(self.layout).shape().center(), // TODO.
                r: 0.0,
            },
            Head::Segbend(head) => {
                if let Some(inner) = self.layout.primitive(head.segbend.bend).inner() {
                    self.bend_circle(inner.into(), width)
                } else {
                    self.dot_circle(
                        self.layout.primitive(head.segbend.bend).core().into(),
                        width,
                    )
                }
            }
        }
    }

    fn bend_circle(&self, bend: BendIndex, width: f64) -> Circle {
        let outer_circle = match bend.primitive(self.layout).shape() {
            Shape::Bend(shape) => shape.outer_circle(),
            _ => unreachable!(),
        };

        Circle {
            pos: outer_circle.pos,
            r: outer_circle.r + width,
        }
    }

    fn dot_circle(&self, dot: DotIndex, width: f64) -> Circle {
        let shape = dot.primitive(self.layout).shape();
        Circle {
            pos: shape.center(),
            r: shape.width() / 2.0 + width + 0.0,
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
            .other_end(head.segbend.dot.into())
    }
}
