use geo::Line;

use crate::{
    graph::{BendIndex, DotIndex},
    layout::Head,
    math::{self, Circle},
    mesh::Mesh,
    rules::{Conditions, Rules},
};

pub struct Guide<'a, 'b> {
    mesh: &'a Mesh,
    rules: &'a Rules,
    conditions: &'b Conditions,
}

impl<'a, 'b> Guide<'a, 'b> {
    pub fn new(mesh: &'a Mesh, rules: &'a Rules, conditions: &'b Conditions) -> Self {
        Self {
            mesh,
            rules,
            conditions,
        }
    }

    pub fn head_into_dot_segment(&self, head: &Head, into: DotIndex, width: f64) -> Line {
        let from_circle = self.head_circle(&head, width);
        let to_circle = Circle {
            pos: self.mesh.primitive(into).weight().circle.pos,
            r: 0.0,
        };

        let from_cw = self.head_cw(&head);
        math::tangent_segment(from_circle, from_cw, to_circle, None)
    }

    pub fn head_around_bend_segment(
        &self,
        head: &Head,
        around: BendIndex,
        cw: bool,
        width: f64,
    ) -> Line {
        let from_circle = self.head_circle(&head, width);
        let to_circle = self.bend_circle(around, width);

        let from_cw = self.head_cw(&head);
        math::tangent_segment(from_circle, from_cw, to_circle, Some(cw))
    }

    pub fn head_around_dot_segment(
        &self,
        head: &Head,
        around: DotIndex,
        cw: bool,
        width: f64,
    ) -> Line {
        let from_circle = self.head_circle(&head, width);
        let to_circle = self.dot_circle(around, width + 5.0);

        let from_cw = self.head_cw(&head);
        math::tangent_segment(from_circle, from_cw, to_circle, Some(cw))
    }

    pub fn head_cw(&self, head: &Head) -> Option<bool> {
        match head.bend {
            Some(bend) => Some(self.mesh.primitive(bend).weight().cw),
            None => None,
        }
    }

    fn head_circle(&self, head: &Head, width: f64) -> Circle {
        let maybe_bend = head.bend;

        let conditions = Conditions {
            lower_net: None,
            higher_net: None,
            layer: None,
            zone: None,
        };

        match maybe_bend {
            Some(bend) => {
                if let Some(inner) = self.mesh.primitive(bend).inner() {
                    self.bend_circle(inner, width)
                } else {
                    self.dot_circle(self.mesh.primitive(bend).core().unwrap(), width + 5.0)
                }
            }
            None => Circle {
                pos: self.mesh.primitive(head.dot).weight().circle.pos,
                r: 0.0,
            },
        }
    }

    fn bend_circle(&self, bend: BendIndex, width: f64) -> Circle {
        let mut circle = self
            .mesh
            .primitive(bend)
            .shape()
            .as_bend()
            .unwrap()
            .circle();
        circle.r += self.rules.ruleset(&self.conditions).clearance.min + 10.0;
        circle
    }

    fn dot_circle(&self, dot: DotIndex, width: f64) -> Circle {
        let circle = self.mesh.primitive(dot).weight().circle;
        Circle {
            pos: circle.pos,
            r: circle.r + width + self.rules.ruleset(self.conditions).clearance.min,
        }
    }
}
