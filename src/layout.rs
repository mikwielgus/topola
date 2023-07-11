use std::cell::{RefCell, Ref};
use std::rc::Rc;
use geo::geometry::Point;

use crate::math::Circle;
use crate::mesh::{Mesh, Index, IndexRTreeWrapper, DotIndex, SegIndex, BendIndex};
use crate::rules::{Rules, Conditions};
use crate::primitive::Primitive;
use crate::weight::{Weight, DotWeight, SegWeight, BendWeight};
use crate::math;


pub struct Layout {
    mesh: Mesh,
    rules: Rules,
}

impl Default for Layout {
    fn default() -> Self {
        return Layout::new();
    }
}

impl Layout {
    pub fn new() -> Self {
        Layout {
            mesh: Mesh::new(),
            rules: Rules::new(),
        }
    }

    pub fn route_around(&mut self, from: DotIndex, around: DotIndex, cw: bool, width: f64) -> DotIndex {
        let from_circle = self.head_guidecircle(from, width);
        let around_circle = self.mesh.weight(Index::Dot(around)).as_dot().unwrap().circle;

        let conditions = Conditions {
            lower_net: None,
            higher_net: None,
            layer: None,
            zone: None,
        };

        let to_circle = self.circle_shell(around_circle, width + 5.0, conditions);

        let maybe_bend = self.mesh.bend(from);
        let from_cw = match maybe_bend {
            Some(bend) => Some(self.mesh.weight(Index::Bend(bend)).as_bend().unwrap().cw),
            None => None,
        };

        let tangent_points = math::tangent_point_pair(from_circle, from_cw, to_circle, Some(cw));

        if maybe_bend.is_some() {
            self.stretch_head_bend(from, tangent_points.0);
        }

        self.route_seg_bend(from, around, tangent_points.1, cw, width)
    }

    pub fn route_to(&mut self, from: DotIndex, to: DotIndex, width: f64) -> DotIndex {
        let from_circle = self.head_guidecircle(from, width);
        
        let conditions = Conditions {
            lower_net: None,
            higher_net: None,
            layer: None,
            zone: None,
        };

        let to_circle = Circle {
            pos: self.mesh.weight(Index::Dot(to)).as_dot().unwrap().circle.pos,
            r: 0.0,
        };

        let maybe_bend = self.mesh.bend(from);
        let from_cw = match maybe_bend {
            Some(bend) => Some(self.mesh.weight(Index::Bend(bend)).as_bend().unwrap().cw),
            None => None,
        };

        let tangent_points = math::tangent_point_pair(from_circle, from_cw, to_circle, None);

        if maybe_bend.is_some() {
            self.stretch_head_bend(from, tangent_points.0);
        }

        self.add_seg(from, to, width);
        to
    }

    fn route_seg_bend(&mut self, from: DotIndex, around: DotIndex, to: Point, cw: bool, width: f64) -> DotIndex {
        let bend_from = self.route_seg(from, to, width);
        let bend_to = self.add_dot(*self.mesh.primitive(Index::Dot(bend_from)).weight.as_dot().unwrap());
        let from_primitive = self.mesh.primitive(Index::Dot(from));
        let net = from_primitive.weight.as_dot().unwrap().net;

        let bend = self.mesh.add_bend(bend_from, bend_to, BendWeight {net, around, cw});
        bend_to
    }

    fn route_seg(&mut self, from: DotIndex, to: Point, width: f64) -> DotIndex {
        let from_primitive = self.mesh.primitive(Index::Dot(from));
        let net = from_primitive.weight.as_dot().unwrap().net;

        assert!(width <= from_primitive.weight.as_dot().unwrap().circle.r * 2.0);

        let to_index = self.mesh.add_dot(DotWeight {
            net,
            circle: Circle {pos: to, r: width / 2.0},
        });
        self.mesh.add_seg(from, to_index, SegWeight {net, width});
        to_index
    }

    fn head_guidecircle(&self, head: DotIndex, width: f64) -> Circle {
        let maybe_bend = self.mesh.bend(head);

        let conditions = Conditions {
            lower_net: None,
            higher_net: None,
            layer: None,
            zone: None,
        };

        match maybe_bend {
            Some(bend) => {
                let head_around = self.mesh.weight(Index::Bend(bend)).as_bend().unwrap().around;
                let circle = self.mesh.weight(Index::Dot(head_around)).as_dot().unwrap().circle;
                self.circle_shell(circle, width + 5.0, conditions)
            },
            None => Circle {
                pos: self.mesh.weight(Index::Dot(head)).as_dot().unwrap().circle.pos,
                r: 0.0,
            },
        }
    }

    fn circle_shell(&self, circle: Circle, width: f64, conditions: Conditions) -> Circle {
        Circle {
            pos: circle.pos,
            r: circle.r + width + self.rules.ruleset(conditions).clearance.min,
        }
    }

    fn stretch_head_bend(&mut self, dot: DotIndex, to: Point) {
        let bend = self.mesh.bend(dot).unwrap();
        let dot_weight = *self.mesh.weight(Index::Dot(dot)).as_dot().unwrap();
        let bend_weight = *self.mesh.weight(Index::Bend(bend)).as_bend().unwrap();

        let fixed_dot: Index = self.mesh.dot_neighbors(Index::Bend(bend))
            .into_iter()
            .filter(|neighbor| *neighbor != Index::Dot(dot))
            .collect::<Vec<Index>>()[0];

        self.mesh.remove_bend(bend);
        self.mesh.remove_dot(dot);

        let new_dot = self.mesh.add_dot(DotWeight {
            net: dot_weight.net,
            circle: Circle {
                pos: to,
                r: dot_weight.circle.r,
            },
        });
        self.mesh.add_bend(*fixed_dot.as_dot().unwrap(), new_dot, bend_weight);
    }

    pub fn add_dot(&mut self, weight: DotWeight) -> DotIndex {
        self.mesh.add_dot(weight)
    }

    pub fn remove_dot(&mut self, index: DotIndex) {
        self.mesh.remove_dot(index);
    }

    pub fn add_seg(&mut self, from: DotIndex, to: DotIndex, width: f64) -> SegIndex {
        let from_primitive = self.mesh.primitive(Index::Dot(from));
        let net = from_primitive.weight.as_dot().unwrap().net;
        self.mesh.add_seg(from, to, SegWeight {net, width})
    }

    pub fn primitives(&self) -> Box<dyn Iterator<Item=Primitive> + '_> {
        self.mesh.primitives()
    }

    pub fn primitive(&self, index: Index) -> Primitive {
        return self.mesh.primitive(index);
    }
}
