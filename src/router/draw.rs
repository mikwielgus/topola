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
        DrawingException, Infringement,
    },
    geometry::{GetLayer, GetSetPos},
    layout::{Layout, LayoutEdit},
    math::{Circle, NoBitangents, RotationSense},
};

#[derive(Error, Debug, Clone, Copy)]
pub enum DrawException {
    #[error(transparent)]
    NoBitangents(#[from] NoBitangents),
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
        sense: RotationSense,
        width: f64,
    ) -> Result<CaneHead, DrawException>;

    fn cane_around_bend(
        &mut self,
        recorder: &mut LayoutEdit,
        head: Head,
        around: BendIndex,
        sense: RotationSense,
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
        let bitangent = self
            .drawing()
            .guide_for_head_into_dot(&head, into, width)
            .map_err(Into::<DrawException>::into)?;

        let (layer, maybe_net) = {
            let face = head.face().primitive(self.drawing());
            (face.layer(), face.maybe_net())
        };

        self.extend_head(
            recorder,
            head,
            bitangent.start_point(),
            |this, recorder| match head.face() {
                DotIndex::Fixed(dot) => this
                    .add_lone_loose_seg(
                        recorder,
                        dot,
                        into,
                        LoneLooseSegWeight(GeneralSegWeight {
                            width,
                            layer,
                            maybe_net,
                        }),
                    )
                    .map(BandTermsegIndex::Lone),
                DotIndex::Loose(dot) => this
                    .add_seq_loose_seg(
                        recorder,
                        into.into(),
                        dot,
                        SeqLooseSegWeight(GeneralSegWeight {
                            width,
                            layer,
                            maybe_net,
                        }),
                    )
                    .map(BandTermsegIndex::Seq),
            },
        )
        .map_err(|err| DrawException::CannotFinishIn(into, err.into()))
    }

    #[debug_ensures(ret.is_ok() -> self.drawing().node_count() == old(self.drawing().node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.drawing().node_count() == old(self.drawing().node_count()))]
    fn cane_around_dot(
        &mut self,
        recorder: &mut LayoutEdit,
        head: Head,
        around: FixedDotIndex,
        sense: RotationSense,
        width: f64,
    ) -> Result<CaneHead, DrawException> {
        let tangent =
            self.drawing()
                .guide_for_head_around_dot(&head, around.into(), sense, width)?;
        let offset =
            self.drawing()
                .offset_for_guide_for_head_around_dot(&head, around.into(), width);

        self.cane_around(
            recorder,
            head,
            around.into(),
            tangent.start_point(),
            tangent.end_point(),
            sense,
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
        sense: RotationSense,
        width: f64,
    ) -> Result<CaneHead, DrawException> {
        let tangent = self
            .drawing()
            .guide_for_head_around_bend(&head, around, sense, width)?;
        let offset = self
            .drawing()
            .offset_for_guide_for_head_around_bend(&head, around, width);

        self.cane_around(
            recorder,
            head,
            around.into(),
            tangent.start_point(),
            tangent.end_point(),
            sense,
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
        sense: RotationSense,
        width: f64,
        offset: f64,
    ) -> Result<CaneHead, DrawingException>;

    fn extend_head<T, E: From<Infringement>>(
        &mut self,
        recorder: &mut LayoutEdit,
        head: Head,
        to: Point,
        then: impl FnOnce(&mut Self, &mut LayoutEdit) -> Result<T, E>,
    ) -> Result<T, E>;

    fn cane(
        &mut self,
        recorder: &mut LayoutEdit,
        head: Head,
        around: GearIndex,
        to: Point,
        sense: RotationSense,
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
        sense: RotationSense,
        width: f64,
        offset: f64,
    ) -> Result<CaneHead, DrawingException> {
        self.extend_head(recorder, head, from, |this, recorder| {
            this.cane(recorder, head, around, to, sense, width, offset)
        })
    }

    fn extend_head<T, E: From<Infringement>>(
        &mut self,
        recorder: &mut LayoutEdit,
        head: Head,
        to: Point,
        then: impl FnOnce(&mut Self, &mut LayoutEdit) -> Result<T, E>,
    ) -> Result<T, E> {
        if let Head::Cane(head) = head {
            let old_pos = self.drawing().geometry().dot_weight(head.face.into()).pos();
            self.move_dot(recorder, head.face.into(), to)?;
            then(self, recorder).inspect_err(|_| {
                // move the head back to where it came from
                self.move_dot(recorder, head.face.into(), old_pos).unwrap();
            })
        } else {
            then(self, recorder)
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
        sense: RotationSense,
        width: f64,
        offset: f64,
    ) -> Result<CaneHead, DrawingException> {
        let layer = head.face().primitive(self.drawing()).layer();
        let maybe_net = head.face().primitive(self.drawing()).maybe_net();

        let dot_weight = LooseDotWeight(GeneralDotWeight {
            circle: Circle {
                pos: to,
                r: width / 2.0,
            },
            layer,
            maybe_net,
        });
        let seg_weight = SeqLooseSegWeight(GeneralSegWeight {
            width,
            layer,
            maybe_net,
        });
        let bend_weight = LooseBendWeight(GeneralBendWeight {
            width,
            offset,
            layer,
            maybe_net,
        });

        // We first try to add cane. If this fails, we try to insert it instead.
        // These two operations differ simply: cane insertion squeezes through
        // under bends, cane addition does not.

        let cane = if let Ok(cane) = self.add_cane(
            recorder,
            head.face(),
            around,
            dot_weight,
            seg_weight,
            bend_weight,
            sense,
        ) {
            cane
        } else {
            self.insert_cane(
                recorder,
                head.face(),
                around,
                dot_weight,
                seg_weight,
                bend_weight,
                sense,
            )?
        };

        Ok(CaneHead {
            face: self.drawing().primitive(cane.bend).other_joint(cane.dot),
            cane,
        })
    }
}
