// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use super::{
    between_vectors_cached, cyclic_breadth_partition_search, dot_product, perp_dot_product,
    RotationSense,
};
use geo::{algorithm::Centroid, Point, Polygon};

#[derive(Clone, Debug, thiserror::Error, PartialEq)]
pub enum PolyTangentException<I> {
    #[error("trying to target empty polygon")]
    EmptyTargetPolygon { origin: Point },

    #[error("invalid polygon tangent arguments")]
    InvalidData {
        poly_ext: Box<[(Point, I)]>,
        origin: Point,
    },
}

/// Caches the `perp_dot_product` call in [`between_vectors`]
#[derive(Clone, Debug)]
pub struct CachedPolyExt<I>(pub Box<[(Point, I, f64)]>);

impl<I: Copy> CachedPolyExt<I> {
    pub fn new(poly_ext: &[(Point, I)], poly_ext_is_cw: bool) -> Self {
        let mut tmp;
        assert!(!poly_ext.len() > 1);
        let poly_ext = if poly_ext_is_cw {
            tmp = poly_ext.to_vec();
            tmp[1..].reverse();
            &tmp[..]
        } else {
            poly_ext
        };

        Self(
            poly_ext
                .iter()
                .enumerate()
                .map(|(i, &(cur, index))| {
                    let prev = poly_ext[(poly_ext.len() + i - 1) % poly_ext.len()].0;
                    let next = poly_ext[(i + 1) % poly_ext.len()].0;
                    let cross = perp_dot_product(cur - prev, cur - next);
                    (cur, index, cross)
                })
                .collect(),
        )
    }

    pub fn centroid(&self) -> Point {
        Polygon::new(self.0.iter().map(|(pt, _, _)| *pt).collect(), Vec::new())
            .centroid()
            .unwrap()
    }

    fn is_outside(&self, i: usize, origin: Point) -> bool {
        let poly_ext = &self.0;
        let len = poly_ext.len();
        let prev = &poly_ext[(len + i - 1) % len];
        let cur = &poly_ext[i];
        let next = &poly_ext[(i + 1) % len];

        // local coordinate system with origin at `cur.0`.
        between_vectors_cached(cur.0 - origin, cur.0 - prev.0, cur.0 - next.0, cur.2)
    }

    fn is_rev_outside(&self, i: usize, origin: Point) -> bool {
        let poly_ext = &self.0;
        let len = poly_ext.len();
        let prev = &poly_ext[(len + i - 1) % len];
        let cur = &poly_ext[i];
        let next = &poly_ext[(i + 1) % len];

        // local coordinate system with origin at `cur.0`.
        between_vectors_cached(origin - cur.0, cur.0 - prev.0, cur.0 - next.0, cur.2)
    }

    /// Calculates the tangents to the polygon exterior going through point `origin`.
    fn tangent_points_intern(&self, origin: Point) -> Option<(usize, usize)> {
        let poly_ext = &self.0;
        let len = poly_ext.len();
        debug_assert!(len > 1);

        // * `pos_false` points to the maximum
        // * `pos_true`  points to the minimum

        // NOTE: although pos_{false,true} are vertex indices, they are actually
        // referring to the "critical" segment(s) (pos_false, pos_false + 1) (and resp. for pos_true).
        // because that is where the `between_vectors` result flips.
        // These critical segments are independent of CW/CCW.

        // if `poly_ext` is oriented CCW, then
        // * `pos_false` will be one too early, and
        // * `pos_true`  will be correct.

        // if `poly_ext` is oriented CW, then
        // * `pos_false` will be correct.
        // * `pos_true`  will be one too early, and

        // In `Self::new` we force CCw.

        let (pos_false, pos_true) = if let (Some(pos_false), Some(pos_true)) =
            cyclic_breadth_partition_search(0..len, |i: usize| self.is_outside(i, origin))
        {
            ((pos_false + 1) % len, pos_true)
        } else if let (Some(rev_pos_false), Some(rev_pos_true)) =
            cyclic_breadth_partition_search(0..len, |i: usize| self.is_rev_outside(i, origin))
        {
            // the following is necessary to find the "furthest" tangent points
            let same_direction = |pos: usize, vec: Point| {
                let vec_cur = origin - poly_ext[pos].0;
                approx::abs_diff_eq!(0., dot_product(vec_cur, vec))
            };
            let mut pos_false = rev_pos_true;
            let vec_false = origin - poly_ext[pos_false].0;
            let mut pos_true = (rev_pos_false + 1) % len;
            let vec_true = origin - poly_ext[pos_true].0;
            // try to move pos_true along CCW
            loop {
                let next_pos = (pos_true + 1) % len;
                if !same_direction(next_pos, vec_true) {
                    break;
                }
                pos_true = next_pos;
            }
            // try to move pos_false along CW
            loop {
                let next_pos = (len + pos_false - 1) % len;
                if !same_direction(next_pos, vec_false) {
                    break;
                }
                pos_false = next_pos;
            }
            (pos_false, pos_true)
        } else {
            return None;
        };

        Some((pos_true, pos_false))
    }

    /// Calculates the tangents to the polygon exterior going through point `origin`.
    pub fn tangent_points(&self, origin: Point) -> Option<(I, I)> {
        self.tangent_points_intern(origin)
            .map(|(pos_min, pos_max)| (self.0[pos_min].1, self.0[pos_max].1))
    }
}

/// Calculates the tangents to the polygon exterior `poly_ext` oriented `cw?=poly_ext_is_cw`
/// going through point `origin`.
pub fn poly_ext_tangent_points<I: Copy>(
    poly_ext: &[(Point, I)],
    poly_ext_is_cw: bool,
    origin: Point,
) -> Result<(I, I), PolyTangentException<I>> {
    if poly_ext.len() < 2 {
        return Err(PolyTangentException::EmptyTargetPolygon { origin });
    }

    CachedPolyExt::new(poly_ext, poly_ext_is_cw)
        .tangent_points(origin)
        .ok_or_else(|| PolyTangentException::InvalidData {
            poly_ext: poly_ext.to_vec().into_boxed_slice(),
            origin,
        })
}

/// Calculates the tangent between the polygons `source` and `target`,
/// according to their intended [`RotationSense`].
pub fn poly_ext_handover<I: Copy>(
    source: &CachedPolyExt<I>,
    source_sense: RotationSense,
    target: &CachedPolyExt<I>,
    target_sense: RotationSense,
) -> Option<(I, I)> {
    use RotationSense::{Clockwise as Cw, Counterclockwise as CoCw};

    let inv_source_sense = match source_sense {
        CoCw => Cw,
        Cw => CoCw,
    };
    let inv_target_sense = match target_sense {
        CoCw => Cw,
        Cw => CoCw,
    };

    // initialization
    let mut pos_trg = {
        let pos = target.tangent_points_intern(source.centroid())?;
        match target_sense {
            CoCw => pos.0,
            Cw => pos.1,
        }
    };
    let mut pos_src = {
        let pos = source.tangent_points_intern(target.centroid())?;
        match inv_source_sense {
            CoCw => pos.0,
            Cw => pos.1,
        }
    };

    // compute flow (direction toward which to shift the positions)
    /*
    let (flow_src, flow_trg) = match (source_sense, target_sense) {
        (CoCw, CoCw) => (Cw, CoCw),
        (Cw, CoCw) => (Cw, Cw),
        (CoCw, Cw) => (CoCw, CoCw),
        (Cw, Cw) => (CoCw, Cw),
    };
    */
    let (flow_src, flow_trg) = (inv_target_sense, source_sense);

    let mut modified = true;
    while modified {
        modified = false;
        if !source.is_outside(pos_src, target.0[pos_trg].0)
            || !source.is_rev_outside(pos_src, target.0[pos_trg].0)
        {
            pos_src = flow_src.step_ccw(pos_src, source.0.len(), 1);
            modified = true;
        }
        if !target.is_outside(pos_trg, source.0[pos_src].0)
            || !target.is_rev_outside(pos_trg, source.0[pos_src].0)
        {
            pos_trg = flow_trg.step_ccw(pos_trg, target.0.len(), 1);
            modified = true;
        }
    }

    // make extremal
    let (xtflow_src, xtflow_trg) = (inv_source_sense, target_sense);
    while modified {
        modified = false;
        let next_pos_src = xtflow_src.step_ccw(pos_src, source.0.len(), 1);
        if source.is_outside(next_pos_src, target.0[pos_trg].0)
            && source.is_rev_outside(next_pos_src, target.0[pos_trg].0)
        {
            pos_src = next_pos_src;
            modified = true;
        }
        let next_pos_trg = xtflow_trg.step_ccw(pos_trg, target.0.len(), 1);
        if target.is_outside(next_pos_trg, source.0[pos_src].0)
            && target.is_rev_outside(next_pos_trg, source.0[pos_src].0)
        {
            pos_trg = next_pos_trg;
            modified = true;
        }
    }

    Some((source.0[pos_src].1, target.0[pos_trg].1))
}

#[cfg(test)]
mod tests {
    use super::{
        poly_ext_handover as pehov, poly_ext_tangent_points as petp, CachedPolyExt,
        RotationSense::{Clockwise as Cw, Counterclockwise as CoCw},
    };
    use crate::drawing::dot::FixedDotIndex;
    use geo::point;

    #[test]
    fn petp00() {
        let poly_ext = &[
            (point! { x: 0., y: 0. }, FixedDotIndex::new(0.into())),
            (point! { x: 1., y: 0. }, FixedDotIndex::new(1.into())),
            (point! { x: 1., y: 1. }, FixedDotIndex::new(2.into())),
            (point! { x: 0., y: 1. }, FixedDotIndex::new(3.into())),
        ];
        let origin = point! { x: 0.5, y: -1.0 };
        assert_eq!(
            petp(poly_ext, false, origin),
            Ok((FixedDotIndex::new(1.into()), FixedDotIndex::new(0.into())))
        );
    }

    #[test]
    fn petp00cw() {
        let poly_ext = &[
            (point! { x: 0., y: 0. }, FixedDotIndex::new(0.into())),
            (point! { x: 0., y: 1. }, FixedDotIndex::new(3.into())),
            (point! { x: 1., y: 1. }, FixedDotIndex::new(2.into())),
            (point! { x: 1., y: 0. }, FixedDotIndex::new(1.into())),
        ];
        let origin = point! { x: 0.5, y: -1.0 };
        assert_eq!(
            petp(poly_ext, true, origin),
            Ok((FixedDotIndex::new(1.into()), FixedDotIndex::new(0.into())))
        );
    }

    #[test]
    fn triangle() {
        let poly_ext = &[
            (point! { x: 0., y: 0. }, FixedDotIndex::new(0.into())),
            (point! { x: 1., y: 1. }, FixedDotIndex::new(1.into())),
            (point! { x: 0., y: 2. }, FixedDotIndex::new(2.into())),
        ];
        let origin = point! { x: 2., y: 1. };
        assert_eq!(
            petp(poly_ext, false, origin),
            Ok((FixedDotIndex::new(2.into()), FixedDotIndex::new(0.into())))
        );
    }

    #[test]
    fn triangle_cw() {
        let poly_ext = &[
            (point! { x: 0., y: 0. }, FixedDotIndex::new(0.into())),
            (point! { x: 0., y: 2. }, FixedDotIndex::new(2.into())),
            (point! { x: 1., y: 1. }, FixedDotIndex::new(1.into())),
        ];
        let origin = point! { x: 2., y: 1. };
        assert_eq!(
            petp(poly_ext, true, origin),
            Ok((FixedDotIndex::new(2.into()), FixedDotIndex::new(0.into())))
        );
    }

    #[test]
    fn handover00() {
        let poly_ext_src = &[
            (point! { x: 4., y: 0. }, FixedDotIndex::new(0.into())),
            (point! { x: 3., y: 3. }, FixedDotIndex::new(1.into())),
            (point! { x: 1., y: 2. }, FixedDotIndex::new(2.into())),
            (point! { x: 1., y: -2. }, FixedDotIndex::new(3.into())),
            (point! { x: 3., y: -3. }, FixedDotIndex::new(4.into())),
        ];
        let source = CachedPolyExt::new(poly_ext_src, false);
        let source = &source;

        let poly_ext_trg = &[
            (point! { x: -4., y: 0. }, FixedDotIndex::new(10.into())),
            (point! { x: -3., y: 3. }, FixedDotIndex::new(11.into())),
            (point! { x: -1., y: 2. }, FixedDotIndex::new(12.into())),
            (point! { x: -1., y: -2. }, FixedDotIndex::new(13.into())),
            (point! { x: -3., y: -3. }, FixedDotIndex::new(14.into())),
        ];
        let target = CachedPolyExt::new(poly_ext_trg, true);
        let target = &target;

        assert_eq!(
            pehov(source, CoCw, target, CoCw),
            Some((FixedDotIndex::new(1.into()), FixedDotIndex::new(11.into())))
        );
        assert_eq!(
            pehov(source, CoCw, target, Cw),
            Some((FixedDotIndex::new(2.into()), FixedDotIndex::new(13.into())))
        );
        assert_eq!(
            pehov(source, Cw, target, CoCw),
            Some((FixedDotIndex::new(3.into()), FixedDotIndex::new(12.into())))
        );
        assert_eq!(
            pehov(source, Cw, target, Cw),
            Some((FixedDotIndex::new(4.into()), FixedDotIndex::new(14.into())))
        );
    }
}
