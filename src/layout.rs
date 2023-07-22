use std::cell::{RefCell, Ref};
use std::rc::Rc;
use geo::geometry::Point;

use crate::math::Circle;
use crate::mesh::{Mesh, TaggedIndex, RTreeWrapper, DotIndex, SegIndex, BendIndex, Tag};
use crate::rules::{Rules, Conditions};
use crate::shape::Shape;
use crate::weight::{TaggedWeight, DotWeight, SegWeight, BendWeight};
use crate::math;

pub struct Layout {
    mesh: Mesh,
    rules: Rules,
}

pub struct Head {
    pub dot: DotIndex,
    pub bend: Option<BendIndex>,
}

impl Layout {
    pub fn new() -> Self {
        Layout {
            mesh: Mesh::new(),
            rules: Rules::new(),
        }
    }

    pub fn route_start(&mut self, from: DotIndex) -> Head {
        Head {dot: from, bend: None}
    }

    pub fn route_end(&mut self, head: Head, to: DotIndex, width: f64) {
        let from_circle = self.head_guidecircle(&head, width);
        
        let conditions = Conditions {
            lower_net: None,
            higher_net: None,
            layer: None,
            zone: None,
        };

        let to_circle = Circle {
            pos: self.mesh.primitive(to).weight().circle.pos,
            r: 0.0,
        };

        let from_cw = self.head_cw(&head);
        let tangent_points = math::tangent_point_pair(from_circle, from_cw, to_circle, None);

        let head = self.extend_head(head, tangent_points.0);
        self.add_seg(head.dot, to, width);
    }

    pub fn route_around_dot(&mut self, head: Head, around: DotIndex, cw: bool, width: f64) -> Head {
        let from_circle = self.head_guidecircle(&head, width);

        let conditions = Conditions {
            lower_net: None,
            higher_net: None,
            layer: None,
            zone: None,
        };

        let to_circle = self.dot_guidecircle(around, width + 5.0, conditions);

        let from_cw = self.head_cw(&head);
        let tangent_points = math::tangent_point_pair(from_circle, from_cw, to_circle, Some(cw));

        let head = self.extend_head(head, tangent_points.0);
        self.route_seg_bend(head, TaggedIndex::Dot(around), tangent_points.1, cw, width)
    }

    pub fn route_around_bend(&mut self, head: Head, around: BendIndex, cw: bool, width: f64) -> Head {
        let from_circle = self.head_guidecircle(&head, width);
        
        let conditions = Conditions {
            lower_net: None,
            higher_net: None,
            layer: None,
            zone: None,
        };

        let to_circle = self.bend_guidecircle(around, width, conditions);

        let from_cw = self.head_cw(&head);
        let tangent_points = math::tangent_point_pair(from_circle, from_cw, to_circle, Some(cw));

        let head = self.extend_head(head, tangent_points.0);
        self.route_seg_bend(head, TaggedIndex::Bend(around), tangent_points.1, cw, width)
    }

    fn route_seg_bend(&mut self, head: Head, around: TaggedIndex, to: Point, cw: bool, width: f64) -> Head {
        let head = self.route_seg(head, to, width);
        let bend_to = self.add_dot(self.mesh.primitive(head.dot).weight());
        let net = self.mesh.primitive(head.dot).weight().net;

        let bend = self.mesh.add_bend(head.dot, bend_to, around, BendWeight {net, cw});
        Head {dot: bend_to, bend: Some(bend)}
    }

    fn route_seg(&mut self, head: Head, to: Point, width: f64) -> Head {
        let net = self.mesh.primitive(head.dot).weight().net;

        assert!(width <= self.mesh.primitive(head.dot).weight().circle.r * 2.0);

        let to_index = self.mesh.add_dot(DotWeight {
            net,
            circle: Circle {pos: to, r: width / 2.0},
        });
        self.mesh.add_seg(head.dot, to_index, SegWeight {net, width});
        Head {dot: to_index, bend: None}
    }

    fn head_guidecircle(&self, head: &Head, width: f64) -> Circle {
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
                    self.bend_guidecircle(inner, width, conditions)
                } else {
                    self.dot_guidecircle(self.mesh.primitive(bend).core().unwrap(), width + 5.0, conditions)
                }
            },
            None => Circle {
                pos: self.mesh.primitive(head.dot).weight().circle.pos,
                r: 0.0,
            },
        }
    }

    fn head_cw(&self, head: &Head) -> Option<bool> {
        match head.bend {
            Some(bend) => Some(self.mesh.primitive(bend).weight().cw),
            None => None,
        }
    }

    fn dot_guidecircle(&self, dot: DotIndex, width: f64, conditions: Conditions) -> Circle {
        let circle = self.mesh.primitive(dot).weight().circle;
        Circle {
            pos: circle.pos,
            r: circle.r + width + self.rules.ruleset(conditions).clearance.min,
        }
    }

    fn bend_guidecircle(&self, bend: BendIndex, width: f64, conditions: Conditions) -> Circle {
        let mut r = width + self.rules.ruleset(conditions).clearance.min;
        let mut layer = bend;

        while let Some(inner) = self.mesh.primitive(layer).inner() {
            r += 5.0 + self.mesh.primitive(inner).shape().width;
            layer = inner;
        }

        let core_circle = self.mesh.primitive(self.mesh.primitive(bend).core().unwrap()).weight().circle;
        Circle {
            pos: core_circle.pos,
            r: core_circle.r + r + 15.0
        }
    }

    fn extend_head(&mut self, head: Head, to: Point) -> Head {
        if let Some(..) = head.bend {
            self.extend_head_bend(head, to)
        } else {
            head
            // No assertion for now because we temporarily use floats.

            //println!("{:?} {:?}", self.mesh.weight(TaggedIndex::Dot(from)).as_dot().unwrap().circle.pos, to);
            //assert!(self.mesh.weight(TaggedIndex::Dot(from)).as_dot().unwrap().circle.pos == to);
        }
    }

    fn extend_head_bend(&mut self, head: Head, to: Point) -> Head {
        let bend = head.bend.unwrap();
        let dot_weight = self.mesh.primitive(head.dot).weight();
        let bend_weight = self.mesh.primitive(bend).weight();
        let around = self.mesh.primitive(bend).around();

        let fixed_dot: DotIndex = self.mesh.primitive(bend).ends()
            .into_iter()
            .filter(|neighbor| {*neighbor != head.dot})
            .collect::<Vec<DotIndex>>()[0];

        self.mesh.remove_bend(bend);
        self.mesh.remove_dot(head.dot);

        let new_dot = self.mesh.add_dot(DotWeight {
            net: dot_weight.net,
            circle: Circle {
                pos: to,
                r: dot_weight.circle.r,
            },
        });

        self.mesh.add_bend(fixed_dot, new_dot, around, bend_weight);
        head
    }

    pub fn add_dot(&mut self, weight: DotWeight) -> DotIndex {
        self.mesh.add_dot(weight)
    }

    pub fn remove_dot(&mut self, index: DotIndex) {
        self.mesh.remove_dot(index);
    }

    pub fn add_seg(&mut self, from: DotIndex, to: DotIndex, width: f64) -> SegIndex {
        let net = self.mesh.primitive(from).weight().net;
        self.mesh.add_seg(from, to, SegWeight {net, width})
    }

    pub fn shapes(&self) -> impl Iterator<Item=Shape> + '_ {
        self.mesh.nodes().map(|ni| untag!(ni, self.mesh.primitive(ni).shape()))
    }
}
