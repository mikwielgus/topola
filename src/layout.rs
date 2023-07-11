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
        let maybe_bend = self.mesh.bend(from);

        let conditions = Conditions {
            lower_net: None,
            higher_net: None,
            layer: None,
            zone: None,
        };

        let from_circle = match maybe_bend {
            Some(bend) => {
                let from_around = self.mesh.weight(Index::Bend(bend)).as_bend().unwrap().around;
                let circle = self.mesh.weight(Index::Dot(from_around)).as_dot().unwrap().circle;
                self.guidecircle(circle, width + 5.0, conditions)
            }
            None => Circle {
                pos: self.mesh.weight(Index::Dot(from)).as_dot().unwrap().circle.pos,
                r: 0.0,
            },
        };
        let around_circle = self.mesh.weight(Index::Dot(around)).as_dot().unwrap().circle;

        let to_circle = self.guidecircle(around_circle, width + 5.0, conditions);
        let tg_pt_pairs = math::tangent_points(from_circle, to_circle);

        for tg_pt_pair in tg_pt_pairs {
            if let Some(bend) = maybe_bend {
                let start_cross = math::cross_product(tg_pt_pair.0, tg_pt_pair.1, from_circle.pos);

                if (self.mesh.weight(Index::Bend(bend)).as_bend().unwrap().cw && start_cross <= 0.0)
                || (!self.mesh.weight(Index::Bend(bend)).as_bend().unwrap().cw && start_cross >= 0.0) {
                    continue;
                }
            }

            let stop_cross = math::cross_product(tg_pt_pair.0, tg_pt_pair.1, to_circle.pos);

            if (cw && stop_cross <= 0.0) || (!cw && stop_cross >= 0.0) {
                continue;
            }

            if maybe_bend.is_some() {
                self.stretch_dangling_bend(from, tg_pt_pair.0);
            }

            return self.route_seg_bend(from, around, tg_pt_pair.1, cw, width);
        }

        unreachable!();
    }

    fn guidecircle(&self, circle: Circle, width: f64, conditions: Conditions) -> Circle {
        Circle {
            pos: circle.pos,
            r: circle.r + width + self.rules.ruleset(conditions).clearance.min,
        }
    }

    fn stretch_dangling_bend(&mut self, dot: DotIndex, to: Point) {
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

    fn route_seg_bend(&mut self, from: DotIndex, around: DotIndex, to: Point, cw: bool, width: f64) -> DotIndex {
        let bend_from = self.route_seg(from, to, width);
        let bend_to = self.add_dot(*self.mesh.primitive(Index::Dot(bend_from)).weight.as_dot().unwrap());
        let from_primitive = self.mesh.primitive(Index::Dot(from));
        let net = from_primitive.weight.as_dot().unwrap().net;

        let bend = self.mesh.add_bend(bend_from, bend_to, BendWeight {net, around, cw});
        bend_to
    }

    pub fn route_seg(&mut self, from: DotIndex, to: Point, width: f64) -> DotIndex {
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
