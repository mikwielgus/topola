use contracts::debug_ensures;
use geo::{EuclideanLength, Point};
use thiserror::Error;

use crate::{
    board::Board,
    layout::{
        bend::{BendIndex, LooseBendWeight},
        dot::{DotIndex, FixedDotIndex, LooseDotIndex, LooseDotWeight},
        graph::{GetNet, MakePrimitive},
        guide::{Guide, Head, HeadTrait, SegbendHead},
        primitive::GetOtherJoint,
        rules::RulesTrait,
        seg::{LoneLooseSegWeight, SeqLooseSegWeight},
        Infringement, LayoutException,
    },
    math::{Circle, NoTangents},
    wraparoundable::WraparoundableIndex,
};

#[derive(Error, Debug, Clone, Copy)]
pub enum DrawException {
    #[error(transparent)]
    NoTangents(#[from] NoTangents),
    // TODO add real error messages + these should eventually use Display
    #[error("cannot finish in {0:?}")]
    CannotFinishIn(FixedDotIndex, #[source] LayoutException),
    #[error("cannot wrap around {0:?}")]
    // neither of the exceptions is the source on its own, might be useful to give them names?
    CannotWrapAround(WraparoundableIndex, LayoutException, LayoutException),
}

pub struct Draw<'a, R: RulesTrait> {
    board: &'a mut Board<R>,
}

impl<'a, R: RulesTrait> Draw<'a, R> {
    pub fn new(board: &'a mut Board<R>) -> Self {
        Self { board }
    }

    pub fn start(&mut self, from: LooseDotIndex) -> Head {
        self.guide().segbend_head(from).into()
    }

    #[debug_ensures(ret.is_ok() -> self.board.layout().node_count() == old(self.board.layout().node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.board.layout().node_count() == old(self.board.layout().node_count()))]
    pub fn finish_in_dot(
        &mut self,
        head: Head,
        into: FixedDotIndex,
        width: f64,
    ) -> Result<(), DrawException> {
        let tangent = self
            .guide()
            .head_into_dot_segment(&head, into, width)
            .map_err(Into::<DrawException>::into)?;
        let head = self
            .extend_head(head, tangent.start_point())
            .map_err(|err| DrawException::CannotFinishIn(into, err.into()))?;
        let net = head.face().primitive(self.board.layout()).net();

        match head.face() {
            DotIndex::Fixed(dot) => {
                self.board
                    .add_lone_loose_seg(dot, into.into(), LoneLooseSegWeight { net, width })
                    .map_err(|err| DrawException::CannotFinishIn(into, err.into()))?;
            }
            DotIndex::Loose(dot) => {
                self.board
                    .add_seq_loose_seg(into.into(), dot, SeqLooseSegWeight { net, width })
                    .map_err(|err| DrawException::CannotFinishIn(into, err.into()))?;
            }
        }
        Ok::<(), DrawException>(())
    }

    #[debug_ensures(ret.is_ok() -> self.board.layout().node_count() == old(self.board.layout().node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.board.layout().node_count() == old(self.board.layout().node_count()))]
    pub fn segbend_around_dot(
        &mut self,
        head: Head,
        around: FixedDotIndex,
        width: f64,
    ) -> Result<SegbendHead, DrawException> {
        let mut tangents = self
            .guide()
            .head_around_dot_segments(&head, around.into(), width)?;
        let offset = self
            .guide()
            .head_around_dot_offset(&head, around.into(), width);
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
                offset,
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

    #[debug_ensures(ret.is_ok() -> self.board.layout().node_count() == old(self.board.layout().node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.board.layout().node_count() == old(self.board.layout().node_count()))]
    pub fn segbend_around_bend(
        &mut self,
        head: Head,
        around: BendIndex,
        width: f64,
    ) -> Result<SegbendHead, DrawException> {
        let mut tangents = self
            .guide()
            .head_around_bend_segments(&head, around.into(), width)?;
        let offset = self
            .guide()
            .head_around_bend_offset(&head, around.into(), width);
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
                offset,
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

    #[debug_ensures(ret.is_ok() -> self.board.layout().node_count() == old(self.board.layout().node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.board.layout().node_count() == old(self.board.layout().node_count()))]
    fn segbend_around(
        &mut self,
        head: Head,
        around: WraparoundableIndex,
        from: Point,
        to: Point,
        cw: bool,
        width: f64,
        offset: f64,
    ) -> Result<SegbendHead, LayoutException> {
        let head = self.extend_head(head, from)?;
        self.segbend(head, around, to, cw, width, offset)
    }

    #[debug_ensures(self.board.layout().node_count() == old(self.board.layout().node_count()))]
    fn extend_head(&mut self, head: Head, to: Point) -> Result<Head, Infringement> {
        if let Head::Segbend(head) = head {
            self.board.move_dot(head.face.into(), to)?;
            Ok(Head::Segbend(head))
        } else {
            Ok(head)
        }
    }

    #[debug_ensures(ret.is_ok() -> self.board.layout().node_count() == old(self.board.layout().node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.board.layout().node_count() == old(self.board.layout().node_count()))]
    fn segbend(
        &mut self,
        head: Head,
        around: WraparoundableIndex,
        to: Point,
        cw: bool,
        width: f64,
        offset: f64,
    ) -> Result<SegbendHead, LayoutException> {
        let net = head.face().primitive(self.board.layout()).net();
        let segbend = self.board.insert_segbend(
            head.face(),
            around,
            LooseDotWeight {
                net,
                circle: Circle {
                    pos: to,
                    r: width / 2.0,
                },
            },
            SeqLooseSegWeight { net, width },
            LooseBendWeight { net, width, offset },
            cw,
        )?;
        Ok::<SegbendHead, LayoutException>(SegbendHead {
            face: self
                .board
                .layout()
                .primitive(segbend.bend)
                .other_joint(segbend.dot),
            segbend,
        })
    }

    #[debug_ensures(ret.is_some() -> self.board.layout().node_count() == old(self.board.layout().node_count() - 4))]
    #[debug_ensures(ret.is_none() -> self.board.layout().node_count() == old(self.board.layout().node_count()))]
    pub fn undo_segbend(&mut self, head: SegbendHead) -> Option<Head> {
        let prev_dot = self
            .board
            .layout()
            .primitive(head.segbend.seg)
            .other_joint(head.segbend.dot.into());

        self.board.remove_segbend(&head.segbend, head.face);
        Some(self.guide().head(prev_dot))
    }

    fn guide(&self) -> Guide<R> {
        Guide::new(self.board.layout())
    }
}
