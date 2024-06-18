use contracts::debug_ensures;
use geo::{EuclideanLength, Point};
use thiserror::Error;

use crate::{
    drawing::{
        band::BandFirstSegIndex,
        bend::{BendIndex, LooseBendWeight},
        dot::{DotIndex, FixedDotIndex, LooseDotIndex, LooseDotWeight},
        graph::{GetLayer, GetMaybeNet, MakePrimitive},
        guide::{CaneHead, Guide, Head, HeadTrait},
        primitive::GetOtherJoint,
        rules::RulesTrait,
        seg::{LoneLooseSegWeight, SeqLooseSegWeight},
        wraparoundable::WraparoundableIndex,
        Infringement, LayoutException,
    },
    layout::Layout,
    math::{Circle, NoTangents},
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
    layout: &'a mut Layout<R>,
}

impl<'a, R: RulesTrait> Draw<'a, R> {
    pub fn new(layout: &'a mut Layout<R>) -> Self {
        Self { layout }
    }

    pub fn start(&mut self, from: LooseDotIndex) -> Head {
        self.guide().cane_head(from).into()
    }

    #[debug_ensures(ret.is_ok() -> self.layout.drawing().node_count() == old(self.layout.drawing().node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.layout.drawing().node_count() == old(self.layout.drawing().node_count()))]
    pub fn finish_in_dot(
        &mut self,
        head: Head,
        into: FixedDotIndex,
        width: f64,
    ) -> Result<BandFirstSegIndex, DrawException> {
        let tangent = self
            .guide()
            .head_into_dot_segment(&head, into, width)
            .map_err(Into::<DrawException>::into)?;
        let head = self
            .extend_head(head, tangent.start_point())
            .map_err(|err| DrawException::CannotFinishIn(into, err.into()))?;
        let layer = head.face().primitive(self.layout.drawing()).layer();
        let maybe_net = head.face().primitive(self.layout.drawing()).maybe_net();

        Ok::<BandFirstSegIndex, DrawException>(match head.face() {
            DotIndex::Fixed(dot) => BandFirstSegIndex::Straight(
                self.layout
                    .add_lone_loose_seg(
                        dot,
                        into.into(),
                        LoneLooseSegWeight {
                            width,
                            layer,
                            maybe_net,
                        },
                    )
                    .map_err(|err| DrawException::CannotFinishIn(into, err.into()))?,
            ),
            DotIndex::Loose(dot) => BandFirstSegIndex::Bended(
                self.layout
                    .add_seq_loose_seg(
                        into.into(),
                        dot,
                        SeqLooseSegWeight {
                            width,
                            layer,
                            maybe_net,
                        },
                    )
                    .map_err(|err| DrawException::CannotFinishIn(into, err.into()))?,
            ),
        })
    }

    #[debug_ensures(ret.is_ok() -> self.layout.drawing().node_count() == old(self.layout.drawing().node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.layout.drawing().node_count() == old(self.layout.drawing().node_count()))]
    pub fn cane_around_dot(
        &mut self,
        head: Head,
        around: FixedDotIndex,
        width: f64,
    ) -> Result<CaneHead, DrawException> {
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
            match self.cane_around(
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

    #[debug_ensures(ret.is_ok() -> self.layout.drawing().node_count() == old(self.layout.drawing().node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.layout.drawing().node_count() == old(self.layout.drawing().node_count()))]
    pub fn cane_around_bend(
        &mut self,
        head: Head,
        around: BendIndex,
        width: f64,
    ) -> Result<CaneHead, DrawException> {
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
            match self.cane_around(
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

    #[debug_ensures(ret.is_ok() -> self.layout.drawing().node_count() == old(self.layout.drawing().node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.layout.drawing().node_count() == old(self.layout.drawing().node_count()))]
    fn cane_around(
        &mut self,
        head: Head,
        around: WraparoundableIndex,
        from: Point,
        to: Point,
        cw: bool,
        width: f64,
        offset: f64,
    ) -> Result<CaneHead, LayoutException> {
        let head = self.extend_head(head, from)?;
        self.cane(head, around, to, cw, width, offset)
    }

    #[debug_ensures(self.layout.drawing().node_count() == old(self.layout.drawing().node_count()))]
    fn extend_head(&mut self, head: Head, to: Point) -> Result<Head, Infringement> {
        if let Head::Cane(head) = head {
            self.layout.move_dot(head.face.into(), to)?;
            Ok(Head::Cane(head))
        } else {
            Ok(head)
        }
    }

    #[debug_ensures(ret.is_ok() -> self.layout.drawing().node_count() == old(self.layout.drawing().node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.layout.drawing().node_count() == old(self.layout.drawing().node_count()))]
    fn cane(
        &mut self,
        head: Head,
        around: WraparoundableIndex,
        to: Point,
        cw: bool,
        width: f64,
        offset: f64,
    ) -> Result<CaneHead, LayoutException> {
        let layer = head.face().primitive(self.layout.drawing()).layer();
        let maybe_net = head.face().primitive(self.layout.drawing()).maybe_net();
        let cane = self.layout.insert_cane(
            head.face(),
            around,
            LooseDotWeight {
                circle: Circle {
                    pos: to,
                    r: width / 2.0,
                },
                layer,
                maybe_net,
            },
            SeqLooseSegWeight {
                width,
                layer,
                maybe_net,
            },
            LooseBendWeight {
                width,
                offset,
                layer,
                maybe_net,
            },
            cw,
        )?;
        Ok::<CaneHead, LayoutException>(CaneHead {
            face: self
                .layout
                .drawing()
                .primitive(cane.bend)
                .other_joint(cane.dot),
            cane,
        })
    }

    #[debug_ensures(ret.is_some() -> self.layout.drawing().node_count() == old(self.layout.drawing().node_count() - 4))]
    #[debug_ensures(ret.is_none() -> self.layout.drawing().node_count() == old(self.layout.drawing().node_count()))]
    pub fn undo_cane(&mut self, head: CaneHead) -> Option<Head> {
        let prev_dot = self
            .layout
            .drawing()
            .primitive(head.cane.seg)
            .other_joint(head.cane.dot.into());

        self.layout.remove_cane(&head.cane, head.face);
        Some(self.guide().head(prev_dot))
    }

    fn guide(&self) -> Guide<impl Copy, R> {
        Guide::new(self.layout.drawing())
    }
}
