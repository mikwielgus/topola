use geo::Point;

use crate::{
    graph::{BendIndex, BendWeight, DotIndex, DotWeight, Ends, SegIndex, SegWeight, TaggedIndex},
    guide::Guide,
    layout::Layout,
    math::Circle,
    rules::{Conditions, Rules},
    segbend::Segbend,
};

#[derive(Debug, Clone, Copy)]
pub struct Head {
    pub dot: DotIndex,
    pub segbend: Option<Segbend>,
}

pub struct Draw<'a> {
    layout: &'a mut Layout,
    rules: &'a Rules,
}

impl<'a> Draw<'a> {
    pub fn new(layout: &'a mut Layout, rules: &'a Rules) -> Self {
        Self { layout, rules }
    }

    pub fn start(&mut self, from: DotIndex) -> Head {
        Head {
            dot: from,
            segbend: self.layout.prev_segbend(from),
        }
    }

    pub fn finish(&mut self, head: Head, into: DotIndex, width: f64) -> Result<(), ()> {
        if let Some(bend) = self.layout.primitive(into).bend() {
            self.finish_in_bend(head, bend, into, width)?;
        } else {
            self.finish_in_dot(head, into, width)?;
        }

        Ok(())
    }

    fn finish_in_dot(&mut self, head: Head, into: DotIndex, width: f64) -> Result<(), ()> {
        let tangent = self
            .guide(&Default::default())
            .head_into_dot_segment(&head, into, width);
        let head = self.extend_head(head, tangent.start_point())?;

        let net = self.layout.primitive(head.dot).weight().net;
        self.layout
            .add_seg(head.dot, into, SegWeight { net, width })?;
        Ok(())
    }

    fn finish_in_bend(
        &mut self,
        head: Head,
        into_bend: BendIndex,
        into: DotIndex,
        width: f64,
    ) -> Result<(), ()> {
        let to_head = Head {
            dot: into,
            segbend: self.layout.next_segbend(into),
        };
        let to_cw = self.guide(&Default::default()).head_cw(&to_head).unwrap();
        let tangent = self
            .guide(&Default::default())
            .head_around_bend_segment(&head, into_bend, to_cw, width);

        let head = self.extend_head(head, tangent.start_point())?;
        let _to_head = self.extend_head(to_head, tangent.end_point())?;

        let net = self.layout.primitive(head.dot).weight().net;
        self.layout
            .add_seg(head.dot, into, SegWeight { net, width })?;
        Ok(())
    }

    pub fn segbend_around_dot(
        &mut self,
        mut head: Head,
        around: DotIndex,
        cw: bool,
        width: f64,
    ) -> Result<Head, ()> {
        let tangent = self
            .guide(&Default::default())
            .head_around_dot_segment(&head, around, cw, width);

        head = self.extend_head(head, tangent.start_point())?;
        self.segbend(
            head,
            TaggedIndex::Dot(around),
            tangent.end_point(),
            cw,
            width,
        )
    }

    pub fn segbend_around_bend(
        &mut self,
        mut head: Head,
        around: BendIndex,
        cw: bool,
        width: f64,
    ) -> Result<Head, ()> {
        let tangent = self
            .guide(&Default::default())
            .head_around_bend_segment(&head, around, cw, width);

        head = self.extend_head(head, tangent.start_point())?;
        self.segbend(
            head,
            TaggedIndex::Bend(around),
            tangent.end_point(),
            cw,
            width,
        )
    }

    fn extend_head(&mut self, head: Head, to: Point) -> Result<Head, ()> {
        if let Some(..) = head.segbend {
            self.extend_head_bend(head, to)
        } else {
            Ok(head)
            // No assertion for now because we temporarily use floats.

            //println!("{:?} {:?}", self.layout.weight(TaggedIndex::Dot(from)).as_dot().unwrap().circle.pos, to);
            //assert!(self.layout.weight(TaggedIndex::Dot(from)).as_dot().unwrap().circle.pos == to);
        }
    }

    fn extend_head_bend(&mut self, head: Head, to: Point) -> Result<Head, ()> {
        self.layout
            .extend_bend(head.segbend.as_ref().unwrap().bend, head.dot, to)?;
        Ok(head)
    }

    fn segbend(
        &mut self,
        head: Head,
        around: TaggedIndex,
        to: Point,
        cw: bool,
        width: f64,
    ) -> Result<Head, ()> {
        let (head, seg) = self.seg(head, to, width)?;
        let dot = head.dot;
        let bend_to = self
            .layout
            .add_dot(self.layout.primitive(head.dot).weight())?;
        let net = self.layout.primitive(head.dot).weight().net;

        let bend = self
            .layout
            .add_bend(head.dot, bend_to, around, BendWeight { net, cw })?;
        Ok(Head {
            dot: bend_to,
            segbend: Some(Segbend { bend, dot, seg }),
        })
    }

    pub fn undo_segbend(&mut self, head: Head) -> Option<Head> {
        let segbend = head.segbend.unwrap();

        self.layout
            .primitive(segbend.ends().0)
            .prev()
            .map(|prev_dot| {
                self.layout.remove_interior(&segbend);

                Head {
                    dot: prev_dot,
                    segbend: self.layout.prev_segbend(prev_dot),
                }
            })
    }

    fn seg(&mut self, head: Head, to: Point, width: f64) -> Result<(Head, SegIndex), ()> {
        let net = self.layout.primitive(head.dot).weight().net;

        assert!(width <= self.layout.primitive(head.dot).weight().circle.r * 2.0);

        let to_index = self.layout.add_dot(DotWeight {
            net,
            circle: Circle {
                pos: to,
                r: width / 2.0,
            },
        })?;
        let seg = self
            .layout
            .add_seg(head.dot, to_index, SegWeight { net, width })?;
        Ok((
            Head {
                dot: to_index,
                segbend: None,
            },
            seg,
        ))
    }

    fn guide(&'a self, conditions: &'a Conditions) -> Guide {
        Guide::new(self.layout, self.rules, conditions)
    }
}
