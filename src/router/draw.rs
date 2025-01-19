// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT
//! router module's routines for drawing and erasing the primitives
//! to pull out or contract the currently routed band.

use contracts_try::debug_ensures;
use geo::Point;
use thiserror::Error;

use crate::{
    drawing::{
        band::BandTermsegIndex,
        bend::{BendIndex, GeneralBendWeight, LooseBendWeight},
        dot::{DotIndex, FixedDotIndex, GeneralDotWeight, LooseDotIndex, LooseDotWeight},
        gear::GearIndex,
        graph::{GetMaybeNet, MakePrimitive},
        head::{CaneHead, GetFace, Head},
        primitive::GetOtherJoint,
        rules::AccessRules,
        seg::{GeneralSegWeight, LoneLooseSegWeight, SeqLooseSegWeight},
        DrawingException, Guide, Infringement,
    },
    geometry::GetLayer,
    layout::{Layout, LayoutEdit},
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

pub trait Draw {
    fn start(&mut self, from: LooseDotIndex) -> Head;

    fn finish_in_dot(
        &mut self,
        recorder: &mut LayoutEdit,
        head: Head,
        into: FixedDotIndex,
        width: f64,
    ) -> Result<BandTermsegIndex, DrawException>;

    fn cane_around_dot(
        &mut self,
        recorder: &mut LayoutEdit,
        head: Head,
        around: FixedDotIndex,
        cw: bool,
        width: f64,
    ) -> Result<CaneHead, DrawException>;

    fn cane_around_bend(
        &mut self,
        recorder: &mut LayoutEdit,
        head: Head,
        around: BendIndex,
        cw: bool,
        width: f64,
    ) -> Result<CaneHead, DrawException>;

    fn undo_cane(&mut self, recorder: &mut LayoutEdit, head: CaneHead) -> Option<Head>;
}

impl<R: AccessRules> Draw for Layout<R> {
    fn start(&mut self, from: LooseDotIndex) -> Head {
        self.drawing().cane_head(from).into()
    }

    #[debug_ensures(ret.is_ok() -> self.drawing().node_count() == old(self.drawing().node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.drawing().node_count() == old(self.drawing().node_count()))]
    fn finish_in_dot(
        &mut self,
        recorder: &mut LayoutEdit,
        head: Head,
        into: FixedDotIndex,
        width: f64,
    ) -> Result<BandTermsegIndex, DrawException> {
        let tangent = self
            .drawing()
            .head_into_dot_segment(&head, into, width)
            .map_err(Into::<DrawException>::into)?;
        let head = self
            .extend_head(recorder, head, tangent.start_point())
            .map_err(|err| DrawException::CannotFinishIn(into, err.into()))?;
        let layer = head.face().primitive(self.drawing()).layer();
        let maybe_net = head.face().primitive(self.drawing()).maybe_net();

        Ok(match head.face() {
            DotIndex::Fixed(dot) => BandTermsegIndex::Straight(
                self.add_lone_loose_seg(
                    recorder,
                    dot,
                    into,
                    LoneLooseSegWeight(GeneralSegWeight {
                        width,
                        layer,
                        maybe_net,
                    }),
                )
                .map_err(|err| DrawException::CannotFinishIn(into, err.into()))?,
            ),
            DotIndex::Loose(dot) => BandTermsegIndex::Bended(
                self.add_seq_loose_seg(
                    recorder,
                    into.into(),
                    dot,
                    SeqLooseSegWeight(GeneralSegWeight {
                        width,
                        layer,
                        maybe_net,
                    }),
                )
                .map_err(|err| DrawException::CannotFinishIn(into, err.into()))?,
            ),
        })
    }

    #[debug_ensures(ret.is_ok() -> self.drawing().node_count() == old(self.drawing().node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.drawing().node_count() == old(self.drawing().node_count()))]
    fn cane_around_dot(
        &mut self,
        recorder: &mut LayoutEdit,
        head: Head,
        around: FixedDotIndex,
        cw: bool,
        width: f64,
    ) -> Result<CaneHead, DrawException> {
        let tangent = self
            .drawing()
            .head_around_dot_segment(&head, around.into(), cw, width)?;
        let offset = self
            .drawing()
            .head_around_dot_offset(&head, around.into(), width);
        self.cane_around(
            recorder,
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

    #[debug_ensures(ret.is_ok() -> self.drawing().node_count() == old(self.drawing().node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.drawing().node_count() == old(self.drawing().node_count()))]
    fn cane_around_bend(
        &mut self,
        recorder: &mut LayoutEdit,
        head: Head,
        around: BendIndex,
        cw: bool,
        width: f64,
    ) -> Result<CaneHead, DrawException> {
        let tangent = self
            .drawing()
            .head_around_bend_segment(&head, around, cw, width)?;
        let offset = self.drawing().head_around_bend_offset(&head, around, width);

        self.cane_around(
            recorder,
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

    #[debug_ensures(ret.is_some() -> self.drawing().node_count() == old(self.drawing().node_count() - 4))]
    #[debug_ensures(ret.is_none() -> self.drawing().node_count() == old(self.drawing().node_count()))]
    fn undo_cane(&mut self, recorder: &mut LayoutEdit, head: CaneHead) -> Option<Head> {
        let prev_dot = self
            .drawing()
            .primitive(head.cane.seg)
            .other_joint(head.cane.dot.into());

        self.remove_cane(recorder, &head.cane, head.face);
        Some(self.drawing().head(prev_dot))
    }
}

trait DrawPrivate {
    type R;

    fn cane_around(
        &mut self,
        recorder: &mut LayoutEdit,
        head: Head,
        around: GearIndex,
        from: Point,
        to: Point,
        cw: bool,
        width: f64,
        offset: f64,
    ) -> Result<CaneHead, DrawingException>;

    fn extend_head(
        &mut self,
        recorder: &mut LayoutEdit,
        head: Head,
        to: Point,
    ) -> Result<Head, Infringement>;

    fn cane(
        &mut self,
        recorder: &mut LayoutEdit,
        head: Head,
        around: GearIndex,
        to: Point,
        cw: bool,
        width: f64,
        offset: f64,
    ) -> Result<CaneHead, DrawingException>;
}

impl<R: AccessRules> DrawPrivate for Layout<R> {
    type R = R;

    #[debug_ensures(ret.is_ok() -> self.drawing().node_count() == old(self.drawing().node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.drawing().node_count() == old(self.drawing().node_count()))]
    fn cane_around(
        &mut self,
        recorder: &mut LayoutEdit,
        head: Head,
        around: GearIndex,
        from: Point,
        to: Point,
        cw: bool,
        width: f64,
        offset: f64,
    ) -> Result<CaneHead, DrawingException> {
        let head = self.extend_head(recorder, head, from)?;
        self.cane(recorder, head, around, to, cw, width, offset)
    }

    #[debug_ensures(self.drawing().node_count() == old(self.drawing().node_count()))]
    fn extend_head(
        &mut self,
        recorder: &mut LayoutEdit,
        head: Head,
        to: Point,
    ) -> Result<Head, Infringement> {
        if let Head::Cane(head) = head {
            self.move_dot(recorder, head.face.into(), to)?;
            Ok(Head::Cane(head))
        } else {
            Ok(head)
        }
    }

    #[debug_ensures(ret.is_ok() -> self.drawing().node_count() == old(self.drawing().node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.drawing().node_count() == old(self.drawing().node_count()))]
    fn cane(
        &mut self,
        recorder: &mut LayoutEdit,
        head: Head,
        around: GearIndex,
        to: Point,
        cw: bool,
        width: f64,
        offset: f64,
    ) -> Result<CaneHead, DrawingException> {
        let layer = head.face().primitive(self.drawing()).layer();
        let maybe_net = head.face().primitive(self.drawing()).maybe_net();
        let cane = self.insert_cane(
            recorder,
            head.face(),
            around,
            LooseDotWeight(GeneralDotWeight {
                circle: Circle {
                    pos: to,
                    r: width / 2.0,
                },
                layer,
                maybe_net,
            }),
            SeqLooseSegWeight(GeneralSegWeight {
                width,
                layer,
                maybe_net,
            }),
            LooseBendWeight(GeneralBendWeight {
                width,
                offset,
                layer,
                maybe_net,
            }),
            cw,
        )?;
        Ok(CaneHead {
            face: self.drawing().primitive(cane.bend).other_joint(cane.dot),
            cane,
        })
    }
}
