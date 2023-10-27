use contracts::{debug_ensures, debug_requires};
use enum_dispatch::enum_dispatch;
use geo::{EuclideanLength, Point};

use crate::{
    graph::{
        BendIndex, DotIndex, FixedBendIndex, FixedBendWeight, FixedDotIndex, FixedDotWeight,
        FixedSegIndex, FixedSegWeight, Index,
    },
    guide::Guide,
    layout::Layout,
    math::Circle,
    primitive::GetWeight,
    rules::{Conditions, Rules},
    segbend::Segbend,
};

#[enum_dispatch]
pub trait HeadTrait {
    fn dot(&self) -> FixedDotIndex;
}

#[enum_dispatch(HeadTrait)]
#[derive(Debug, Clone, Copy)]
pub enum Head {
    Bare(BareHead),
    Segbend(SegbendHead),
}

#[derive(Debug, Clone, Copy)]
pub struct BareHead {
    pub dot: FixedDotIndex,
}

impl HeadTrait for BareHead {
    fn dot(&self) -> FixedDotIndex {
        self.dot
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SegbendHead {
    pub dot: FixedDotIndex,
    pub segbend: Segbend,
}

impl HeadTrait for SegbendHead {
    fn dot(&self) -> FixedDotIndex {
        self.dot
    }
}

pub struct Draw<'a> {
    layout: &'a mut Layout,
    rules: &'a Rules,
}

impl<'a> Draw<'a> {
    pub fn new(layout: &'a mut Layout, rules: &'a Rules) -> Self {
        Self { layout, rules }
    }

    pub fn start(&mut self, from: FixedDotIndex) -> Head {
        self.head(from)
    }

    #[debug_ensures(ret.is_ok() -> self.layout.node_count() == old(self.layout.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.layout.node_count() == old(self.layout.node_count()))]
    pub fn finish(&mut self, head: Head, into: FixedDotIndex, width: f64) -> Result<(), ()> {
        if let Some(bend) = self.layout.primitive(into).bend() {
            self.finish_in_bend(head, bend, into, width)?;
        } else {
            self.finish_in_dot(head, into, width)?;
        }
        Ok(())
    }

    #[debug_ensures(ret.is_ok() -> self.layout.node_count() == old(self.layout.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.layout.node_count() == old(self.layout.node_count()))]
    fn finish_in_dot(&mut self, head: Head, into: FixedDotIndex, width: f64) -> Result<(), ()> {
        let tangent = self
            .guide(&Default::default())
            .head_into_dot_segment(&head, into, width)?;
        let head = self.extend_head(head, tangent.start_point())?;

        let net = self.layout.primitive(head.dot()).weight().net;
        self.layout
            .add_fixed_seg(head.dot(), into, FixedSegWeight { net, width })?;
        Ok(())
    }

    #[debug_ensures(ret.is_ok() -> self.layout.node_count() == old(self.layout.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.layout.node_count() == old(self.layout.node_count()))]
    fn finish_in_bend(
        &mut self,
        head: Head,
        into_bend: FixedBendIndex,
        into: FixedDotIndex,
        width: f64,
    ) -> Result<(), ()> {
        let to_head = self.head(into);
        let to_cw = self.guide(&Default::default()).head_cw(&to_head).unwrap();
        let tangent = self.guide(&Default::default()).head_around_bend_segment(
            &head,
            into_bend.into(),
            to_cw,
            width,
        )?;

        let head = self.extend_head(head, tangent.start_point())?;
        let _to_head = self.extend_head(to_head, tangent.end_point())?;

        let net = self.layout.primitive(head.dot()).weight().net;
        self.layout
            .add_fixed_seg(head.dot(), into, FixedSegWeight { net, width })?;
        Ok(())
    }

    #[debug_ensures(ret.is_ok() -> self.layout.node_count() == old(self.layout.node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.layout.node_count() == old(self.layout.node_count()))]
    pub fn segbend_around_dot(
        &mut self,
        head: Head,
        around: DotIndex,
        width: f64,
    ) -> Result<SegbendHead, ()> {
        let mut tangents = self
            .guide(&Default::default())
            .head_around_dot_segments(&head, around, width)?;
        let mut dirs = [true, false];

        if tangents.1.euclidean_length() < tangents.0.euclidean_length() {
            tangents = (tangents.1, tangents.0);
            dirs = [false, true];
        }

        [tangents.0, tangents.1]
            .iter()
            .enumerate()
            .find_map(|(i, tangent)| {
                self.segbend_around(
                    head,
                    around.into(),
                    tangent.start_point(),
                    tangent.end_point(),
                    dirs[i],
                    width,
                )
                .ok()
            })
            .ok_or(())
    }

    #[debug_ensures(ret.is_ok() -> self.layout.node_count() == old(self.layout.node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.layout.node_count() == old(self.layout.node_count()))]
    pub fn segbend_around_bend(
        &mut self,
        head: Head,
        around: BendIndex,
        width: f64,
    ) -> Result<SegbendHead, ()> {
        let mut tangents = self.guide(&Default::default()).head_around_bend_segments(
            &head,
            around.into(),
            width,
        )?;
        let mut dirs = [true, false];

        if tangents.1.euclidean_length() < tangents.0.euclidean_length() {
            tangents = (tangents.1, tangents.0);
            dirs = [false, true];
        }

        [tangents.0, tangents.1]
            .iter()
            .enumerate()
            .find_map(|(i, tangent)| {
                self.segbend_around(
                    head,
                    around.into(),
                    tangent.start_point(),
                    tangent.end_point(),
                    dirs[i],
                    width,
                )
                .ok()
            })
            .ok_or(())
    }

    #[debug_ensures(ret.is_ok() -> self.layout.node_count() == old(self.layout.node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.layout.node_count() == old(self.layout.node_count()))]
    fn segbend_around(
        &mut self,
        head: Head,
        around: Index,
        from: Point,
        to: Point,
        cw: bool,
        width: f64,
    ) -> Result<SegbendHead, ()> {
        let head = self.extend_head(head, from)?;
        self.segbend(head, around, to, cw, width)
    }

    #[debug_ensures(self.layout.node_count() == old(self.layout.node_count()))]
    fn extend_head(&mut self, head: Head, to: Point) -> Result<Head, ()> {
        if let Head::Segbend(head) = head {
            self.layout.move_dot(head.dot, to)?;

            /*if let TaggedIndex::Dot(around) = self.layout.primitive(head.segbend.bend).around() {
                let cw = self.layout.primitive(head.segbend.bend).weight().cw;
                let prev_dot = self.layout.primitive(head.segbend.ends().0).prev().unwrap();
                let prev_head = self.prev_head(prev_dot);

                let alternate_tangent = self
                    .guide(&Default::default())
                    .head_around_dot_segment(&prev_head, around, cw, 5.0)?;

                let segbend_dot_pos = self.layout.primitive(head.segbend.dot).weight().circle.pos;

                if alternate_tangent.end_point().euclidean_distance(&to)
                    < segbend_dot_pos.euclidean_distance(&to)
                {
                    self.layout.flip_bend(head.segbend.bend);
                    self.layout
                        .move_dot(head.segbend.dot, alternate_tangent.end_point())?;
                }
            }*/

            Ok(Head::Segbend(head))
        } else {
            Ok(head)
            // No assertion for now because we temporarily use floats.

            //println!("{:?} {:?}", self.layout.weight(TaggedIndex::Dot(from)).as_dot().unwrap().circle.pos, to);
            //assert!(self.layout.weight(TaggedIndex::Dot(from)).as_dot().unwrap().circle.pos == to);
        }
    }

    #[debug_ensures(ret.is_ok() -> self.layout.node_count() == old(self.layout.node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.layout.node_count() == old(self.layout.node_count()))]
    fn segbend(
        &mut self,
        head: Head,
        around: Index,
        to: Point,
        cw: bool,
        width: f64,
    ) -> Result<SegbendHead, ()> {
        let (head, seg) = self.seg(head, to, width)?;
        let dot = head.dot;
        let bend_to = self
            .layout
            .add_fixed_dot(self.layout.primitive(head.dot).weight())
            .map_err(|err| {
                self.undo_seg(head, seg);
                err
            })?;

        let net = self.layout.primitive(head.dot).weight().net;

        let bend = self
            .layout
            .add_fixed_bend(
                head.dot,
                bend_to,
                around,
                FixedBendWeight { net, width, cw },
            )
            .map_err(|err| {
                self.layout.remove(bend_to.into());
                self.undo_seg(head, seg);
                err
            })?;
        Ok(SegbendHead {
            dot: bend_to,
            segbend: Segbend { bend, dot, seg },
        })
    }

    #[debug_ensures(ret.is_some() -> self.layout.node_count() == old(self.layout.node_count() - 4))]
    #[debug_ensures(ret.is_none() -> self.layout.node_count() == old(self.layout.node_count()))]
    pub fn undo_segbend(&mut self, head: SegbendHead) -> Option<Head> {
        let prev_dot = self
            .layout
            .primitive(head.segbend.seg)
            .other_end(head.segbend.dot);

        self.layout.remove_interior(&head.segbend);
        self.layout.remove(head.dot().into());

        Some(self.head(prev_dot))
    }

    #[debug_requires(width <= self.layout.primitive(head.dot()).weight().circle.r * 2.0)]
    #[debug_ensures(ret.is_ok() -> self.layout.node_count() == old(self.layout.node_count() + 2))]
    #[debug_ensures(ret.is_err() -> self.layout.node_count() == old(self.layout.node_count()))]
    fn seg(&mut self, head: Head, to: Point, width: f64) -> Result<(BareHead, FixedSegIndex), ()> {
        let net = self.layout.primitive(head.dot()).weight().net;
        let to_index = self.layout.add_fixed_dot(FixedDotWeight {
            net,
            circle: Circle {
                pos: to,
                r: width / 2.0,
            },
        })?;
        let seg = self
            .layout
            .add_fixed_seg(head.dot(), to_index, FixedSegWeight { net, width })
            .map_err(|err| {
                self.layout.remove(to_index.into());
                err
            })?;
        Ok((BareHead { dot: to_index }, seg))
    }

    #[debug_ensures(self.layout.node_count() == old(self.layout.node_count() - 2))]
    fn undo_seg(&mut self, head: BareHead, seg: FixedSegIndex) {
        self.layout.remove(seg.into());
        self.layout.remove(head.dot.into());
    }

    fn head(&self, dot: FixedDotIndex) -> Head {
        if let Some(segbend) = self.layout.segbend(dot) {
            SegbendHead { dot, segbend }.into()
        } else {
            BareHead { dot }.into()
        }
    }

    fn guide(&'a self, conditions: &'a Conditions) -> Guide {
        Guide::new(self.layout, self.rules, conditions)
    }
}
