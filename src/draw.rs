use contracts::debug_ensures;
use enum_dispatch::enum_dispatch;
use geo::{EuclideanLength, Point};

use crate::{
    graph::{
        BendIndex, DotIndex, FixedDotIndex, FixedSegWeight, GetBand, GetNet, Index, LooseBendIndex,
        LooseBendWeight, LooseDotIndex, LooseDotWeight, LooseSegWeight, MakePrimitive,
        WraparoundableIndex,
    },
    guide::Guide,
    layout::Layout,
    math::Circle,
    primitive::{GetOtherEnd, GetWeight},
    rules::{Conditions, Rules},
    segbend::Segbend,
};

#[enum_dispatch]
pub trait HeadTrait {
    fn dot(&self) -> DotIndex;
    fn band(&self) -> usize;
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
    pub band: usize,
}

impl HeadTrait for BareHead {
    fn dot(&self) -> DotIndex {
        self.dot.into()
    }

    fn band(&self) -> usize {
        self.band
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SegbendHead {
    pub dot: LooseDotIndex,
    pub segbend: Segbend,
    pub band: usize,
}

impl HeadTrait for SegbendHead {
    fn dot(&self) -> DotIndex {
        self.dot.into()
    }

    fn band(&self) -> usize {
        self.band
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

    pub fn start(&mut self, from: LooseDotIndex) -> Head {
        self.segbend_head(from).into()
    }

    #[debug_ensures(ret.is_ok() -> self.layout.node_count() == old(self.layout.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.layout.node_count() == old(self.layout.node_count()))]
    pub fn finish_in_dot(&mut self, head: Head, into: FixedDotIndex, width: f64) -> Result<(), ()> {
        let tangent = self
            .guide(&Default::default())
            .head_into_dot_segment(&head, into, width)?;
        let head = self.extend_head(head, tangent.start_point())?;

        let net = head.dot().primitive(self.layout).net();

        match head.dot() {
            DotIndex::Fixed(dot) => {
                self.layout
                    .add_fixed_seg(into.into(), dot, FixedSegWeight { net, width })?;
            }
            DotIndex::Loose(dot) => {
                self.layout.add_loose_seg(
                    into.into(),
                    dot,
                    LooseSegWeight { band: head.band() },
                )?;
            }
        }
        Ok(())
    }

    #[debug_ensures(ret.is_ok() -> self.layout.node_count() == old(self.layout.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.layout.node_count() == old(self.layout.node_count()))]
    pub fn finish_in_bend(
        &mut self,
        head: Head,
        into_bend: LooseBendIndex,
        into: LooseDotIndex,
        width: f64,
    ) -> Result<(), ()> {
        let to_head = self.segbend_head(into);
        let to_cw = self
            .guide(&Default::default())
            .head_cw(&to_head.into())
            .unwrap();
        let tangent = self.guide(&Default::default()).head_around_bend_segment(
            &head,
            into_bend.into(),
            to_cw,
            width,
        )?;

        let head = self.extend_head(head, tangent.start_point())?;
        let _to_head = self.extend_head(to_head.into(), tangent.end_point())?;

        let _net = head.dot().primitive(self.layout).net();
        self.layout.add_loose_seg(
            head.dot(),
            into.into(),
            LooseSegWeight { band: head.band() },
        )?;
        Ok(())
    }

    #[debug_ensures(ret.is_ok() -> self.layout.node_count() == old(self.layout.node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.layout.node_count() == old(self.layout.node_count()))]
    pub fn segbend_around_dot(
        &mut self,
        head: Head,
        around: FixedDotIndex,
        width: f64,
    ) -> Result<SegbendHead, ()> {
        let mut tangents = self.guide(&Default::default()).head_around_dot_segments(
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
        around: WraparoundableIndex,
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
            Ok(Head::Segbend(head))
        } else {
            Ok(head)
        }
    }

    #[debug_ensures(ret.is_ok() -> self.layout.node_count() == old(self.layout.node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.layout.node_count() == old(self.layout.node_count()))]
    fn segbend(
        &mut self,
        head: Head,
        around: WraparoundableIndex,
        to: Point,
        cw: bool,
        width: f64,
    ) -> Result<SegbendHead, ()> {
        let segbend = self.layout.add_segbend(
            head.dot(),
            around,
            LooseDotWeight {
                band: head.band(),
                circle: Circle {
                    pos: to,
                    r: width / 2.0,
                },
            },
            LooseSegWeight { band: head.band() },
            LooseBendWeight {
                band: head.band(),
                offset: 3.0,
                cw,
            },
        )?;
        Ok(SegbendHead {
            dot: self.layout.primitive(segbend.bend).other_end(segbend.dot),
            segbend,
            band: head.band(),
        })
    }

    #[debug_ensures(ret.is_some() -> self.layout.node_count() == old(self.layout.node_count() - 4))]
    #[debug_ensures(ret.is_none() -> self.layout.node_count() == old(self.layout.node_count()))]
    pub fn undo_segbend(&mut self, head: SegbendHead) -> Option<Head> {
        let prev_dot = self
            .layout
            .primitive(head.segbend.seg)
            .other_end(head.segbend.dot.into());
        let band = head.band;

        self.layout.remove_interior(&head.segbend);
        self.layout.remove(head.dot().into());

        Some(self.head(prev_dot, band))
    }

    #[debug_ensures(self.layout.node_count() == old(self.layout.node_count()))]
    pub fn update_bow(&mut self, _bend: LooseBendIndex) {
        /*let cw = self.layout.primitive(bend).weight().cw;
        let ends = self.layout.primitive(bend).ends();
        let from_head = self.rear_head(ends.0);
        let to_head = self.rear_head(ends.1);

        let from = self
            .guide(&Default::default())
            .head_around_bend_segment(&from_head, inner, cw, 3.0);
        let to = self
            .guide(&Default::default())
            .head_around_bend_segment(&from_head, inner, cw, 3.0);
        self.layout.reposition_bend(bend, from, to);*/
    }

    fn head(&self, dot: DotIndex, band: usize) -> Head {
        match dot {
            DotIndex::Fixed(loose) => BareHead { dot: loose, band }.into(),
            DotIndex::Loose(fixed) => self.segbend_head(fixed).into(),
        }
    }

    fn segbend_head(&self, dot: LooseDotIndex) -> SegbendHead {
        SegbendHead {
            dot,
            segbend: self.layout.segbend(dot),
            band: self.layout.primitive(dot).weight().band(),
        }
    }

    fn rear_head(&self, dot: LooseDotIndex) -> Head {
        self.head(
            self.rear(self.segbend_head(dot)),
            self.layout.primitive(dot).weight().band(),
        )
    }

    fn rear(&self, head: SegbendHead) -> DotIndex {
        self.layout
            .primitive(head.segbend.seg)
            .other_end(head.segbend.dot.into())
    }

    fn guide(&'a self, conditions: &'a Conditions) -> Guide {
        Guide::new(self.layout, self.rules, conditions)
    }
}
