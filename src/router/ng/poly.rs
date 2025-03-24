// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::Point;
use specctra_core::rules::AccessRules;

use crate::{
    drawing::{
        band::BandUid,
        dot::FixedDotIndex,
        graph::{MakePrimitive as _, PrimitiveIndex},
        head::{CaneHead, Head},
        primitive::MakePrimitiveShape as _,
    },
    geometry::{compound::ManageCompounds, shape::AccessShape as _, GetSetPos as _},
    graph::{GenericIndex, GetPetgraphIndex as _},
    layout::{poly::PolyWeight, CompoundEntryLabel, Layout, LayoutEdit},
    math::{is_poly_convex_hull_cw, CachedPolyExt, RotationSense},
    router::ng::{
        pie::{mayrev, utils::rotate_iter},
        EvalException,
    },
};

#[derive(Clone, Debug)]
// the entry point is where `active_head` ends
// TODO: two-phase initialization via separate types
pub struct PolygonRouting {
    pub idx: GenericIndex<PolyWeight>,

    pub apex: FixedDotIndex,

    pub convex_hull: CachedPolyExt<FixedDotIndex>,

    pub entry_point: Option<FixedDotIndex>,

    pub inner: Option<BandUid>,

    pub cw: RotationSense,
}

impl PolygonRouting {
    /// calculates the convex hull of a poly exterior
    pub fn new<R>(
        layout: &Layout<R>,
        cw: RotationSense,
        polyidx: GenericIndex<PolyWeight>,
        apex: FixedDotIndex,
    ) -> Self {
        let convex_hull = layout
            .drawing()
            .geometry()
            .compound_members(GenericIndex::new(polyidx.petgraph_index()))
            .filter_map(|(entry_label, primitive_node)| {
                let PrimitiveIndex::FixedDot(poly_dot) = primitive_node else {
                    return None;
                };

                if apex == poly_dot {
                    None
                } else {
                    Some((
                        layout
                            .drawing()
                            .geometry()
                            .dot_weight(poly_dot.into())
                            .pos(),
                        entry_label,
                        poly_dot,
                    ))
                }
            })
            .filter(|(_, entry_label, _)| *entry_label == CompoundEntryLabel::Normal)
            .map(|(pt, _, idx)| (pt, idx))
            .collect::<Vec<_>>();

        // `poly_ext` is convex, so any pivot point is okay
        let ext_is_cw = is_poly_convex_hull_cw(&convex_hull[..], 1);

        Self {
            idx: polyidx,
            apex,
            convex_hull: CachedPolyExt::new(&convex_hull[..], ext_is_cw),
            entry_point: None,
            inner: None,
            cw,
        }
    }

    pub fn center<R: AccessRules>(&self, layout: &Layout<R>) -> Point {
        self.apex.primitive(layout.drawing()).shape().center()
    }

    /// calculate the entry or exit point for the polygon (set `invert_cw` to `true` for exit point)
    pub fn entry_point(
        &self,
        destination: Point,
        invert_cw: bool,
    ) -> Result<FixedDotIndex, EvalException> {
        // note that the left-most point has the greatest angle (measured counter-clockwise)
        let Some((lhs, rhs)) = self.convex_hull.tangent_points(destination) else {
            return Err(EvalException::InvalidPolyTangentData {
                poly_ext: self.convex_hull.clone(),
                origin: destination,
            });
        };
        let cw = match self.cw {
            RotationSense::Counterclockwise => false,
            RotationSense::Clockwise => true,
        };
        Ok(if invert_cw ^ cw { lhs } else { rhs })
    }

    fn route_next<R: AccessRules>(
        &self,
        layout: &mut Layout<R>,
        recorder: &mut LayoutEdit,
        route_length: &mut f64,
        old_head: Head,
        ext_core: FixedDotIndex,
        width: f64,
    ) -> Result<CaneHead, EvalException> {
        super::cane_around(
            layout,
            recorder,
            route_length,
            old_head,
            ext_core,
            self.inner,
            self.cw,
            width,
        )
    }

    pub fn route_to_entry<R: AccessRules>(
        &self,
        layout: &mut Layout<R>,
        recorder: &mut LayoutEdit,
        old_head: Head,
        entry_point: FixedDotIndex,
        width: f64,
    ) -> Result<(Head, f64), EvalException> {
        let mut route_length = 0.0;
        let active_head = Head::Cane(self.route_next(
            layout,
            recorder,
            &mut route_length,
            old_head,
            entry_point,
            width,
        )?);
        Ok((active_head, route_length))
    }

    pub fn route_to_exit<R: AccessRules>(
        &self,
        layout: &mut Layout<R>,
        recorder: &mut LayoutEdit,
        mut active_head: Head,
        exit: FixedDotIndex,
        width: f64,
    ) -> Result<(Head, f64), EvalException> {
        let old_entry = self.entry_point.unwrap();
        if old_entry == exit {
            // nothing to do
            return Ok((active_head, 0.0));
        }
        log::debug!(
            "route_to_exit on {:?} from {:?} to {:?} sense {:?}",
            self.apex,
            old_entry,
            exit,
            self.cw,
        );
        let mut mr = mayrev::MaybeReversed::new(&self.convex_hull.0[..]);
        // the convex hull is oriented counter-clockwise
        // FIXME(fogti): I have no clue where the orientation gets wrong...
        mr.reversed = !match self.cw {
            RotationSense::Counterclockwise => false,
            RotationSense::Clockwise => true,
        };

        let mut route_length = 0.0;

        for pdot in rotate_iter(Iterator::map(mr.iter(), |(_, pdot, _)| *pdot), |&pdot| {
            pdot == old_entry
        })
        .1
        .skip(1)
        {
            // route along the origin polygon
            active_head = Head::Cane(self.route_next(
                layout,
                recorder,
                &mut route_length,
                active_head,
                pdot,
                width,
            )?);
            if pdot == exit {
                break;
            }
        }
        Ok((active_head, route_length))
    }
}
