use contracts::debug_ensures;

use geo::{EuclideanLength, Point};

use crate::{
    graph::{
        BendIndex, DotIndex, FixedDotIndex, FixedSegWeight, GetBand, GetNet, LooseBendIndex,
        LooseBendWeight, LooseDotIndex, LooseDotWeight, LooseSegWeight, MakePrimitive,
        WraparoundableIndex,
    },
    guide::{Guide, Head, HeadTrait, SegbendHead},
    layout::{Infringement, Layout, LayoutException},
    math::{Circle, NoTangents},
    primitive::GetOtherEnd,
    rules::{Conditions, Rules},
};

#[derive(Debug, Clone, Copy)]
pub enum DrawException {
    NoTangents(NoTangents),
    CannotFinishIn(FixedDotIndex, LayoutException),
    CannotWrapAround(WraparoundableIndex, LayoutException, LayoutException),
}

impl From<NoTangents> for DrawException {
    fn from(err: NoTangents) -> Self {
        DrawException::NoTangents(err)
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
        self.guide(&Default::default()).segbend_head(from).into()
    }

    #[debug_ensures(ret.is_ok() -> self.layout.node_count() == old(self.layout.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.layout.node_count() == old(self.layout.node_count()))]
    pub fn finish_in_dot(
        &mut self,
        head: Head,
        into: FixedDotIndex,
        width: f64,
    ) -> Result<(), DrawException> {
        let tangent = self
            .guide(&Default::default())
            .head_into_dot_segment(&head, into, width)
            .map_err(Into::<DrawException>::into)?;
        let head = self
            .extend_head(head, tangent.start_point())
            .map_err(|err| DrawException::CannotFinishIn(into, err.into()))?;

        let net = head.face().primitive(self.layout).net();

        match head.face() {
            DotIndex::Fixed(dot) => {
                self.layout
                    .add_fixed_seg(into.into(), dot, FixedSegWeight { net, width })
                    .map_err(|err| DrawException::CannotFinishIn(into, err.into()))?;
            }
            DotIndex::Loose(dot) => {
                self.layout
                    .add_loose_seg(into.into(), dot, LooseSegWeight { band: head.band() })
                    .map_err(|err| DrawException::CannotFinishIn(into, err.into()))?;
            }
        }
        Ok::<(), DrawException>(())
    }

    #[debug_ensures(ret.is_ok() -> self.layout.node_count() == old(self.layout.node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.layout.node_count() == old(self.layout.node_count()))]
    pub fn segbend_around_dot(
        &mut self,
        head: Head,
        around: FixedDotIndex,
        width: f64,
    ) -> Result<SegbendHead, DrawException> {
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

        let mut errs = vec![];

        for (i, tangent) in [tangents.0, tangents.1].iter().enumerate() {
            match self.segbend_around(
                head,
                around.into(),
                tangent.start_point(),
                tangent.end_point(),
                dirs[i],
                width,
            ) {
                Ok(ok) => return Ok(ok),
                Err(err) => errs.push(err),
            }
        }

        Err(DrawException::CannotWrapAround(
            around.into(),
            errs[0],
            errs[1],
        ))
    }

    #[debug_ensures(ret.is_ok() -> self.layout.node_count() == old(self.layout.node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.layout.node_count() == old(self.layout.node_count()))]
    pub fn segbend_around_bend(
        &mut self,
        head: Head,
        around: BendIndex,
        width: f64,
    ) -> Result<SegbendHead, DrawException> {
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

        let mut errs = vec![];

        for (i, tangent) in [tangents.0, tangents.1].iter().enumerate() {
            match self.segbend_around(
                head,
                around.into(),
                tangent.start_point(),
                tangent.end_point(),
                dirs[i],
                width,
            ) {
                Ok(ok) => return Ok(ok),
                Err(err) => errs.push(err),
            }
        }

        Err(DrawException::CannotWrapAround(
            around.into(),
            errs[0],
            errs[1],
        ))
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
    ) -> Result<SegbendHead, LayoutException> {
        let head = self.extend_head(head, from)?;
        self.segbend(head, around, to, cw, width)
    }

    #[debug_ensures(self.layout.node_count() == old(self.layout.node_count()))]
    fn extend_head(&mut self, head: Head, to: Point) -> Result<Head, Infringement> {
        if let Head::Segbend(head) = head {
            self.layout.move_dot(head.face, to)?;
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
    ) -> Result<SegbendHead, LayoutException> {
        let segbend = self.layout.insert_segbend(
            head.face(),
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
        Ok::<SegbendHead, LayoutException>(SegbendHead {
            face: self.layout.primitive(segbend.bend).other_end(segbend.dot),
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

        self.layout.remove_segbend(&head.segbend, head.face);
        Some(self.guide(&Default::default()).head(prev_dot, band))
    }

    fn guide(&'a self, conditions: &'a Conditions) -> Guide {
        Guide::new(self.layout, self.rules, conditions)
    }
}
