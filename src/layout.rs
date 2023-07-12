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

    pub fn route_around_dot(&mut self, from: DotIndex, around: DotIndex, cw: bool, width: f64) -> DotIndex {
        let from_circle = self.head_guidecircle(from, width);

        let conditions = Conditions {
            lower_net: None,
            higher_net: None,
            layer: None,
            zone: None,
        };

        let to_circle = self.dot_guidecircle(around, width + 5.0, conditions);

        let from_cw = self.mesh.cw(Index::Dot(from));
        let tangent_points = math::tangent_point_pair(from_circle, from_cw, to_circle, Some(cw));

        self.extend_head(from, tangent_points.0);
        self.route_seg_bend(from, Index::Dot(around), tangent_points.1, cw, width)
    }

    pub fn route_around_bend(&mut self, from: DotIndex, around: BendIndex, cw: bool, width: f64) -> DotIndex {
        let from_circle = self.head_guidecircle(from, width);
        
        let conditions = Conditions {
            lower_net: None,
            higher_net: None,
            layer: None,
            zone: None,
        };

        let to_circle = self.bend_guidecircle(around, width, conditions);

        let from_cw = self.mesh.cw(Index::Dot(from));
        let tangent_points = math::tangent_point_pair(from_circle, from_cw, to_circle, Some(cw));

        self.extend_head(from, tangent_points.0);
        self.route_seg_bend(from, Index::Bend(around), tangent_points.1, cw, width)
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

        let from_cw = self.mesh.cw(Index::Dot(from));
        let tangent_points = math::tangent_point_pair(from_circle, from_cw, to_circle, None);

        self.extend_head(from, tangent_points.0);
        self.add_seg(from, to, width);
        to
    }

    fn route_seg_bend(&mut self, from: DotIndex, around: Index, to: Point, cw: bool, width: f64) -> DotIndex {
        let bend_from = self.route_seg(from, to, width);
        let bend_to = self.add_dot(*self.mesh.primitive(Index::Dot(bend_from)).weight.as_dot().unwrap());
        let from_primitive = self.mesh.primitive(Index::Dot(from));
        let net = from_primitive.weight.as_dot().unwrap().net;

        let mut layer = around;
        while let Index::Bend(..) = layer {
            layer = self.mesh.weight(layer).as_bend().unwrap().around;
        }
        let center = *layer.as_dot().unwrap();

        let bend = self.mesh.add_bend(bend_from, bend_to, BendWeight {net, around, center, cw});
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
                
                match self.mesh.weight(head_around) {
                    Weight::Dot(..) => self.dot_guidecircle(*head_around.as_dot().unwrap(), width + 5.0, conditions),
                    Weight::Bend(..) => self.bend_guidecircle(*head_around.as_bend().unwrap(), width, conditions),
                    Weight::Seg(..) => unreachable!(),
                }
            },
            None => Circle {
                pos: self.mesh.weight(Index::Dot(head)).as_dot().unwrap().circle.pos,
                r: 0.0,
            },
        }
    }

    fn dot_guidecircle(&self, dot: DotIndex, width: f64, conditions: Conditions) -> Circle {
        let circle = self.mesh.weight(Index::Dot(dot)).as_dot().unwrap().circle;
        Circle {
            pos: circle.pos,
            r: circle.r + width + self.rules.ruleset(conditions).clearance.min,
        }
    }

    fn bend_guidecircle(&self, bend: BendIndex, width: f64, conditions: Conditions) -> Circle {
        let mut layer = Index::Bend(bend);
        let mut r = width + self.rules.ruleset(conditions).clearance.min;

        while let Index::Bend(..) = layer {
            layer = self.mesh.weight(layer).as_bend().unwrap().around;
            r += 5.0 + self.mesh.primitive(layer).width();
        }

        let circle = self.primitive(layer).weight.as_dot().unwrap().circle;
        Circle {
            pos: circle.pos,
            r: circle.r - 5.0 + r,
        }
    }

    fn extend_head(&mut self, from: DotIndex, to: Point) {
        if let Some(..) = self.mesh.bend(from) {
            self.extend_head_bend(from, to);
        } else {
            // No assertion for now because we temporarily use floats.

            //println!("{:?} {:?}", self.mesh.weight(Index::Dot(from)).as_dot().unwrap().circle.pos, to);
            //assert!(self.mesh.weight(Index::Dot(from)).as_dot().unwrap().circle.pos == to);
        }
    }

    fn extend_head_bend(&mut self, dot: DotIndex, to: Point) {
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

    pub fn bend(&self, index: DotIndex) -> Option<BendIndex> {
        return self.mesh.bend(index);
    }
}
