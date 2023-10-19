use geo::Line;

use crate::{
    draw::{Head, HeadTrait, SegbendHead},
    graph::{BendIndex, DotIndex},
    layout::Layout,
    math::{self, Circle},
    primitive::MakeShape,
    rules::{Conditions, Rules},
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
        into: DotIndex,
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
                pos: self.layout.primitive(head.dot()).weight().circle.pos,
                r: 0.0,
            },
            Head::Segbend(head) => {
                if let Some(inner) = self.layout.primitive(head.segbend.bend).inner() {
                    self.bend_circle(inner, width)
                } else {
                    self.dot_circle(
                        self.layout.primitive(head.segbend.bend).core().unwrap(),
                        width,
                    )
                }
            }
        }
    }

    fn bend_circle(&self, bend: BendIndex, _width: f64) -> Circle {
        let mut circle = self
            .layout
            .primitive(bend)
            .shape()
            .into_bend()
            .unwrap()
            .circle();
        circle.r += self.rules.ruleset(&self.conditions).clearance.min;
        circle
    }

    fn dot_circle(&self, dot: DotIndex, width: f64) -> Circle {
        let circle = self.layout.primitive(dot).weight().circle;
        Circle {
            pos: circle.pos,
            r: circle.r + width + self.rules.ruleset(self.conditions).clearance.min,
        }
    }
}
