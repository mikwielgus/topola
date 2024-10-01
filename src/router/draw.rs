use contracts::debug_ensures;
use geo::Point;
use thiserror::Error;

use crate::{
    drawing::{
        band::BandTermsegIndex,
        bend::{BendIndex, LooseBendWeight},
        dot::{DotIndex, FixedDotIndex, LooseDotIndex, LooseDotWeight},
        gear::GearIndex,
        graph::{GetLayer, GetMaybeNet, MakePrimitive},
        guide::Guide,
        head::{CaneHead, GetFace, Head},
        primitive::GetOtherJoint,
        rules::AccessRules,
        seg::{LoneLooseSegWeight, SeqLooseSegWeight},
        DrawingException, Infringement,
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
    CannotFinishIn(FixedDotIndex, #[source] DrawingException),
    #[error("cannot wrap around {0:?}")]
    CannotWrapAround(GearIndex, #[source] DrawingException),
}

pub struct Draw<'a, R: AccessRules> {
    layout: &'a mut Layout<R>,
}

impl<'a, R: AccessRules> Draw<'a, R> {
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
    ) -> Result<BandTermsegIndex, DrawException> {
        let tangent = self
            .guide()
            .head_into_dot_segment(&head, into, width)
            .map_err(Into::<DrawException>::into)?;
        let head = self
            .extend_head(head, tangent.start_point())
            .map_err(|err| DrawException::CannotFinishIn(into, err.into()))?;
        let layer = head.face().primitive(self.layout.drawing()).layer();
        let maybe_net = head.face().primitive(self.layout.drawing()).maybe_net();

        Ok(match head.face() {
            DotIndex::Fixed(dot) => BandTermsegIndex::Straight(
                self.layout
                    .add_lone_loose_seg(
                        dot,
                        into,
                        LoneLooseSegWeight {
                            width,
                            layer,
                            maybe_net,
                        },
                    )
                    .map_err(|err| DrawException::CannotFinishIn(into, err.into()))?,
            ),
            DotIndex::Loose(dot) => BandTermsegIndex::Bended(
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
        cw: bool,
        width: f64,
    ) -> Result<CaneHead, DrawException> {
        let tangent = self
            .guide()
            .head_around_dot_segment(&head, around.into(), cw, width)?;
        let offset = self
            .guide()
            .head_around_dot_offset(&head, around.into(), width);
        self.cane_around(
            head,
            around.into(),
            tangent.start_point(),
            tangent.end_point(),
            cw,
            width,
            offset,
        )
        .map_err(|err| DrawException::CannotWrapAround(around.into(), err))
    }

    #[debug_ensures(ret.is_ok() -> self.layout.drawing().node_count() == old(self.layout.drawing().node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.layout.drawing().node_count() == old(self.layout.drawing().node_count()))]
    pub fn cane_around_bend(
        &mut self,
        head: Head,
        around: BendIndex,
        cw: bool,
        width: f64,
    ) -> Result<CaneHead, DrawException> {
        let tangent = self
            .guide()
            .head_around_bend_segment(&head, around, cw, width)?;
        let offset = self
            .guide()
            .head_around_bend_offset(&head, around, width);

        self.cane_around(
            head,
            around.into(),
            tangent.start_point(),
            tangent.end_point(),
            cw,
            width,
            offset,
        )
        .map_err(|err| DrawException::CannotWrapAround(around.into(), err))
    }

    #[debug_ensures(ret.is_ok() -> self.layout.drawing().node_count() == old(self.layout.drawing().node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.layout.drawing().node_count() == old(self.layout.drawing().node_count()))]
    fn cane_around(
        &mut self,
        head: Head,
        around: GearIndex,
        from: Point,
        to: Point,
        cw: bool,
        width: f64,
        offset: f64,
    ) -> Result<CaneHead, DrawingException> {
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
        around: GearIndex,
        to: Point,
        cw: bool,
        width: f64,
        offset: f64,
    ) -> Result<CaneHead, DrawingException> {
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
        Ok(CaneHead {
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
