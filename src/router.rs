use geo::geometry::Point;
use std::cell::{Ref, RefCell};
use std::rc::Rc;

use crate::graph::{BendIndex, DotIndex, Path, SegIndex, TaggedIndex};
use crate::graph::{BendWeight, DotWeight, SegWeight, TaggedWeight};
use crate::guide::Guide;
use crate::layout::Layout;
use crate::math;
use crate::math::Circle;
use crate::mesh::Mesh;
use crate::rules::{Conditions, Rules};
use crate::shape::Shape;

pub struct Router {
    pub layout: Layout,
    mesh: Mesh,
    rules: Rules,
}

pub struct Head {
    pub dot: DotIndex,
    pub bend: Option<BendIndex>,
}

impl Router {
    pub fn new() -> Self {
        Router {
            layout: Layout::new(),
            mesh: Mesh::new(),
            rules: Rules::new(),
        }
    }

    pub fn draw_start(&mut self, from: DotIndex) -> Head {
        Head {
            dot: from,
            bend: self.layout.primitive(from).bend(),
        }
    }

    pub fn draw_finish(&mut self, head: Head, into: DotIndex, width: f64) -> Result<(), ()> {
        if let Some(bend) = self.layout.primitive(into).bend() {
            self.draw_finish_in_bend(head, bend, into, width)
        } else {
            self.draw_finish_in_dot(head, into, width)
        }
    }

    fn draw_finish_in_dot(&mut self, head: Head, into: DotIndex, width: f64) -> Result<(), ()> {
        let tangent = self
            .guide(&Default::default())
            .head_into_dot_segment(&head, into, width);
        let head = self.extend_head(head, tangent.start_point())?;

        let net = self.layout.primitive(head.dot).weight().net;
        self.layout
            .add_seg(head.dot, into, SegWeight { net, width })?;
        self.mesh.triangulate(&self.layout);
        Ok(())
    }

    fn draw_finish_in_bend(
        &mut self,
        head: Head,
        into_bend: BendIndex,
        into: DotIndex,
        width: f64,
    ) -> Result<(), ()> {
        let to_head = Head {
            bend: Some(into_bend),
            dot: into,
        };
        let to_cw = self.guide(&Default::default()).head_cw(&to_head).unwrap();
        let tangent = self
            .guide(&Default::default())
            .head_around_bend_segment(&head, into_bend, to_cw, width);

        let head = self.extend_head(head, tangent.start_point())?;
        let to_head = self.extend_head(to_head, tangent.end_point())?;

        let net = self.layout.primitive(head.dot).weight().net;
        self.layout
            .add_seg(head.dot, into, SegWeight { net, width })?;
        self.mesh.triangulate(&self.layout);
        Ok(())
    }

    pub fn squeeze_around_dot(
        &mut self,
        head: Head,
        around: DotIndex,
        cw: bool,
        width: f64,
    ) -> Result<Head, ()> {
        let outer = self.layout.primitive(around).outer().unwrap();
        let head = self.draw_around_dot(head, around, cw, width)?;
        self.layout.reattach_bend(outer, head.bend.unwrap());

        self.reroute_outward(outer)?;
        Ok(head)
    }

    pub fn draw_around_dot(
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
        self.draw_seg_bend(
            head,
            TaggedIndex::Dot(around),
            tangent.end_point(),
            cw,
            width,
        )
    }

    pub fn squeeze_around_bend(
        &mut self,
        head: Head,
        around: BendIndex,
        cw: bool,
        width: f64,
    ) -> Result<Head, ()> {
        let outer = self.layout.primitive(around).outer().unwrap();
        let head = self.draw_around_bend(head, around, cw, width)?;
        self.layout.reattach_bend(outer, head.bend.unwrap());

        self.reroute_outward(outer)?;
        Ok(head)
    }

    pub fn draw_around_bend(
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
        self.draw_seg_bend(
            head,
            TaggedIndex::Bend(around),
            tangent.end_point(),
            cw,
            width,
        )
    }

    fn draw_seg_bend(
        &mut self,
        head: Head,
        around: TaggedIndex,
        to: Point,
        cw: bool,
        width: f64,
    ) -> Result<Head, ()> {
        let head = self.draw_seg(head, to, width)?;
        let bend_to = self
            .layout
            .add_dot(self.layout.primitive(head.dot).weight())?;
        let net = self.layout.primitive(head.dot).weight().net;

        let bend = self
            .layout
            .add_bend(head.dot, bend_to, around, BendWeight { net, cw })?;
        self.mesh.triangulate(&self.layout);
        Ok(Head {
            dot: bend_to,
            bend: Some(bend),
        })
    }

    fn reroute_outward(&mut self, bend: BendIndex) -> Result<(), ()> {
        let mut endss: Vec<[DotIndex; 2]> = vec![];
        let mut interiors: Vec<Vec<TaggedIndex>> = vec![];
        let cw = self.layout.primitive(bend).weight().cw;

        let mut cur_bend = bend;
        loop {
            let bow = self.layout.bow(cur_bend);
            endss.push(bow.ends());
            interiors.push(bow.interior());

            cur_bend = match self.layout.primitive(cur_bend).outer() {
                Some(new_bend) => new_bend,
                None => break,
            }
        }

        let core = self.layout.primitive(bend).core().unwrap();
        let mut maybe_inner = self.layout.primitive(bend).inner();

        for interior in interiors {
            self.layout.remove_open_set(interior);
        }

        for ends in endss {
            let mut head = self.draw_start(ends[0]);
            let width = 5.0;

            if let Some(inner) = maybe_inner {
                head = self.draw_around_bend(head, inner, cw, width)?;
            } else {
                head = self.draw_around_dot(head, core, cw, width)?;
            }

            maybe_inner = head.bend;
            self.draw_finish(head, ends[1], width)?;
            self.relax_band(maybe_inner.unwrap());
        }

        Ok(())
    }

    fn draw_seg(&mut self, head: Head, to: Point, width: f64) -> Result<Head, ()> {
        let net = self.layout.primitive(head.dot).weight().net;

        assert!(width <= self.layout.primitive(head.dot).weight().circle.r * 2.);

        let to_index = self.layout.add_dot(DotWeight {
            net,
            circle: Circle {
                pos: to,
                r: width / 2.0,
            },
        })?;
        self.layout
            .add_seg(head.dot, to_index, SegWeight { net, width })?;
        self.mesh.triangulate(&self.layout);
        Ok(Head {
            dot: to_index,
            bend: None,
        })
    }

    fn relax_band(&mut self, bend: BendIndex) {
        let mut prev_bend = bend;
        while let Some(cur_bend) = self.layout.primitive(prev_bend).prev_akin() {
            if self.layout.primitive(cur_bend).cross_product() >= 0. {
                self.release_bow(cur_bend);
            }

            prev_bend = cur_bend;
        }

        let mut prev_bend = bend;
        while let Some(cur_bend) = self.layout.primitive(prev_bend).next_akin() {
            if self.layout.primitive(cur_bend).cross_product() >= 0. {
                self.release_bow(cur_bend);
            }

            prev_bend = cur_bend;
        }
    }

    fn release_bow(&mut self, bend: BendIndex) {
        let bow = self.layout.bow(bend);
        let ends = bow.ends();

        self.layout.remove_open_set(bow.interior());

        let head = self.draw_start(ends[0]);
        let _ = self.draw_finish(head, ends[1], 5.);
    }

    pub fn move_dot(&mut self, dot: DotIndex, to: Point) -> Result<(), ()> {
        self.layout.move_dot(dot, to)?;

        if let Some(outer) = self.layout.primitive(dot).outer() {
            self.reroute_outward(outer)?;
        }

        self.mesh.triangulate(&self.layout);
        Ok(())
    }

    fn extend_head(&mut self, head: Head, to: Point) -> Result<Head, ()> {
        if let Some(..) = head.bend {
            self.extend_head_bend(head, to)
        } else {
            Ok(head)
            // No assertion for now because we temporarily use floats.

            //println!("{:?} {:?}", self.layout.weight(TaggedIndex::Dot(from)).as_dot().unwrap().circle.pos, to);
            //assert!(self.layout.weight(TaggedIndex::Dot(from)).as_dot().unwrap().circle.pos == to);
        }
    }

    fn extend_head_bend(&mut self, head: Head, to: Point) -> Result<Head, ()> {
        self.layout.extend_bend(head.bend.unwrap(), head.dot, to)?;
        Ok(head)
    }

    fn guide<'a>(&'a self, conditions: &'a Conditions) -> Guide {
        Guide::new(&self.layout, &self.rules, conditions)
    }

    pub fn routeedges(&self) -> impl Iterator<Item = (Point, Point)> + '_ {
        self.mesh.edges().map(|endpoints| {
            let index0 = endpoints.0;
            let index1 = endpoints.1;
            (
                self.layout.primitive(index0).shape().center(),
                self.layout.primitive(index1).shape().center(),
            )
        })
    }
}