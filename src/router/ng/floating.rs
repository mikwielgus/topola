// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::Point;

use crate::{
    drawing::{
        dot::DotIndex,
        graph::MakePrimitive as _,
        primitive::{GetWeight as _, Primitive},
        rules::AccessRules,
    },
    geometry::GetSetPos as _,
    layout::Layout,
    math::Tunnel,
    router::ng::EvalException,
};

/// floating edges don't count, instead, only the end points around the streak
/// get connected directly (but the intermediates shouldn't cross any vertices)
#[derive(Clone, Copy, Debug)]
pub struct FloatingRouting {
    // the starting point of the floating routing
    // (we just use the position of the head face)
    //pub origin: Point,
    /// the circle segment / tunnel in which we are allowed to navigate
    pub tunnel: Tunnel,
}

impl FloatingRouting {
    pub fn new<R: AccessRules>(
        layout: &Layout<R>,
        active_head_face: DotIndex,
        lhs: Point,
        rhs: Point,
    ) -> Self {
        let active_head_pos = match active_head_face.primitive(layout.drawing()) {
            Primitive::FixedDot(dot) => dot.weight().0,
            Primitive::LooseDot(dot) => dot.weight().0,
            _ => unreachable!(),
        }
        .pos();

        Self {
            tunnel: Tunnel(lhs - active_head_pos, rhs - active_head_pos),
        }
    }

    pub fn push(self, othr: &Self, active_head_face: DotIndex) -> Result<Self, EvalException> {
        Ok(Self {
            tunnel: self.tunnel.intersection(&othr.tunnel).ok_or(
                EvalException::FloatingEmptyTunnel {
                    origin: active_head_face,
                },
            )?,
        })
    }
}
