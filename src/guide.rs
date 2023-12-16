use geo::Line;

use crate::{
    draw::{Head, HeadTrait},
    graph::{BendIndex, DotIndex, FixedDotIndex, MakePrimitive},
    layout::Layout,
    math::{self, Circle},
    primitive::{GetCore, GetInnerOuter, GetWeight, MakeShape},
    rules::{Conditions, Rules},
    shape::{Shape, ShapeTrait},
};

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
    ) -> Result<Line, ()> {
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
    ) -> Result<(Line, Line), ()> {
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
    ) -> Result<Line, ()> {
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
    ) -> Result<(Line, Line), ()> {
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
    ) -> Result<Line, ()> {
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
                pos: head.dot().primitive(self.layout).shape().center(), // TODO.
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
            r: shape.width() / 2.0 + width + self.rules.ruleset(self.conditions).clearance.min,
        }
    }
}
