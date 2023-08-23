use geo::geometry::Point;
use std::cell::{Ref, RefCell};
use std::rc::Rc;

use crate::graph::{BendIndex, DotIndex, Path, SegIndex, TaggedIndex};
use crate::graph::{BendWeight, DotWeight, SegWeight, TaggedWeight};
use crate::guide::Guide;
use crate::math;
use crate::math::Circle;
use crate::mesh::Mesh;
use crate::rules::{Conditions, Rules};
use crate::shape::Shape;

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
        Head {
            dot: from,
            bend: self.mesh.primitive(from).bend(),
        }
    }

    pub fn route_finish(&mut self, head: Head, into: DotIndex, width: f64) -> Result<(), ()> {
        if let Some(bend) = self.mesh.primitive(into).bend() {
            self.route_finish_in_bend(head, bend, into, width)
        } else {
            self.route_finish_in_dot(head, into, width)
        }
    }

    fn route_finish_in_dot(&mut self, head: Head, into: DotIndex, width: f64) -> Result<(), ()> {
        let tangent = self
            .guide(&Default::default())
            .head_into_dot_segment(&head, into, width);
        let head = self.extend_head(head, tangent.start_point())?;
        self.add_seg(head.dot, into, width)?;
        Ok(())
    }

    fn route_finish_in_bend(
        &mut self,
        head: Head,
        to_bend: BendIndex,
        to: DotIndex,
        width: f64,
    ) -> Result<(), ()> {
        let to_head = Head {
            bend: Some(to_bend),
            dot: to,
        };
        let to_cw = self.guide(&Default::default()).head_cw(&to_head).unwrap();
        let tangent = self
            .guide(&Default::default())
            .head_around_bend_segment(&head, to_bend, to_cw, width);
        let head = self.extend_head(head, tangent.start_point())?;

        let to_head = self.extend_head(to_head, tangent.end_point())?;
        self.add_seg(head.dot, to, width)?;
        Ok(())
    }

    pub fn shove_around_dot(
        &mut self,
        head: Head,
        around: DotIndex,
        cw: bool,
        width: f64,
    ) -> Result<Head, ()> {
        let outer = self.mesh.primitive(around).outer().unwrap();
        let head = self.route_around_dot(head, around, cw, width)?;
        self.mesh.reattach_bend(outer, head.bend.unwrap());

        self.reroute_outward(outer)?;
        Ok(head)
    }

    pub fn route_around_dot(
        &mut self,
        head: Head,
        around: DotIndex,
        cw: bool,
        width: f64,
    ) -> Result<Head, ()> {
        let tangent = self
            .guide(&Default::default())
            .head_around_dot_segment(&head, around, cw, width);

        let head = self.extend_head(head, tangent.start_point())?;
        self.route_seg_bend(
            head,
            TaggedIndex::Dot(around),
            tangent.end_point(),
            cw,
            width,
        )
    }

    pub fn shove_around_bend(
        &mut self,
        head: Head,
        around: BendIndex,
        cw: bool,
        width: f64,
    ) -> Result<Head, ()> {
        let outer = self.mesh.primitive(around).outer().unwrap();
        let head = self.route_around_bend(head, around, cw, width)?;
        self.mesh.reattach_bend(outer, head.bend.unwrap());

        self.reroute_outward(outer)?;
        Ok(head)
    }

    pub fn route_around_bend(
        &mut self,
        head: Head,
        around: BendIndex,
        cw: bool,
        width: f64,
    ) -> Result<Head, ()> {
        let tangent = self
            .guide(&Default::default())
            .head_around_bend_segment(&head, around, cw, width);

        let head = self.extend_head(head, tangent.start_point())?;
        self.route_seg_bend(
            head,
            TaggedIndex::Bend(around),
            tangent.end_point(),
            cw,
            width,
        )
    }

    fn route_seg_bend(
        &mut self,
        head: Head,
        around: TaggedIndex,
        to: Point,
        cw: bool,
        width: f64,
    ) -> Result<Head, ()> {
        let head = self.route_seg(head, to, width)?;
        let bend_to = self.add_dot(self.mesh.primitive(head.dot).weight())?;
        let net = self.mesh.primitive(head.dot).weight().net;

        let bend = self
            .mesh
            .add_bend(head.dot, bend_to, around, BendWeight { net, cw })?;
        Ok(Head {
            dot: bend_to,
            bend: Some(bend),
        })
    }

    fn reroute_outward(&mut self, bend: BendIndex) -> Result<(), ()> {
        let mut endss: Vec<[DotIndex; 2]> = vec![];
        let mut interiors: Vec<Vec<TaggedIndex>> = vec![];
        let cw = self.mesh.primitive(bend).weight().cw;

        let mut cur_bend = bend;
        loop {
            let bow = self.mesh.bow(cur_bend);
            endss.push(bow.ends());
            interiors.push(bow.interior());

            cur_bend = match self.mesh.primitive(cur_bend).outer() {
                Some(new_bend) => new_bend,
                None => break,
            }
        }

        let core = self.mesh.primitive(bend).core().unwrap();
        let mut maybe_inner = self.mesh.primitive(bend).inner();

        for interior in interiors {
            self.mesh.remove_open_set(interior);
        }

        for ends in endss {
            let mut head = self.route_start(ends[0]);
            let width = 5.0;

            if let Some(inner) = maybe_inner {
                head = self.route_around_bend(head, inner, cw, width)?;
            } else {
                head = self.route_around_dot(head, core, cw, width)?;
            }

            maybe_inner = head.bend;
            self.route_finish(head, ends[1], width)?;
            self.relax_band(maybe_inner.unwrap());
        }

        Ok(())
    }

    fn route_seg(&mut self, head: Head, to: Point, width: f64) -> Result<Head, ()> {
        let net = self.mesh.primitive(head.dot).weight().net;

        assert!(width <= self.mesh.primitive(head.dot).weight().circle.r * 2.);

        let to_index = self.mesh.add_dot(DotWeight {
            net,
            circle: Circle {
                pos: to,
                r: width / 2.0,
            },
        })?;
        self.add_seg(head.dot, to_index, width)?;
        Ok(Head {
            dot: to_index,
            bend: None,
        })
    }

    fn relax_band(&mut self, bend: BendIndex) {
        let mut prev_bend = bend;
        while let Some(cur_bend) = self.mesh.primitive(prev_bend).prev_akin() {
            if self.mesh.primitive(cur_bend).cross_product() >= 0. {
                self.release_bow(cur_bend);
            }

            prev_bend = cur_bend;
        }

        let mut prev_bend = bend;
        while let Some(cur_bend) = self.mesh.primitive(prev_bend).next_akin() {
            if self.mesh.primitive(cur_bend).cross_product() >= 0. {
                self.release_bow(cur_bend);
            }

            prev_bend = cur_bend;
        }
    }

    fn release_bow(&mut self, bend: BendIndex) {
        let bow = self.mesh.bow(bend);
        let ends = bow.ends();

        self.mesh.remove_open_set(bow.interior());

        let head = self.route_start(ends[0]);
        let _ = self.route_finish(head, ends[1], 5.);
    }

    pub fn move_dot(&mut self, dot: DotIndex, to: Point) -> Result<(), ()> {
        self.mesh.move_dot(dot, to)?;

        if let Some(outer) = self.mesh.primitive(dot).outer() {
            self.reroute_outward(outer)?;
        }

        Ok(())
    }

    fn extend_head(&mut self, head: Head, to: Point) -> Result<Head, ()> {
        if let Some(..) = head.bend {
            self.extend_head_bend(head, to)
        } else {
            Ok(head)
            // No assertion for now because we temporarily use floats.

            //println!("{:?} {:?}", self.mesh.weight(TaggedIndex::Dot(from)).as_dot().unwrap().circle.pos, to);
            //assert!(self.mesh.weight(TaggedIndex::Dot(from)).as_dot().unwrap().circle.pos == to);
        }
    }

    fn extend_head_bend(&mut self, head: Head, to: Point) -> Result<Head, ()> {
        self.mesh.extend_bend(head.bend.unwrap(), head.dot, to)?;
        Ok(head)
    }

    pub fn add_dot(&mut self, weight: DotWeight) -> Result<DotIndex, ()> {
        self.mesh.add_dot(weight)
    }

    pub fn add_seg(&mut self, from: DotIndex, to: DotIndex, width: f64) -> Result<SegIndex, ()> {
        let net = self.mesh.primitive(from).weight().net;
        self.mesh.add_seg(from, to, SegWeight { net, width })
    }

    pub fn shapes(&self) -> impl Iterator<Item = Shape> + '_ {
        self.mesh
            .nodes()
            .map(|ni| untag!(ni, self.mesh.primitive(ni).shape()))
    }

    pub fn edges(&self) -> impl Iterator<Item = (Point, Point)> + '_ {
        self.mesh.edges().map(|endpoints| {
            let index0 = endpoints.0;
            let index1 = endpoints.1;
            (
                untag!(index0, self.mesh.primitive(index0).shape().center()),
                untag!(index1, self.mesh.primitive(index1).shape().center()),
            )
        })
    }

    fn guide<'a>(&'a self, conditions: &'a Conditions) -> Guide {
        Guide::new(&self.mesh, &self.rules, conditions)
    }
}
