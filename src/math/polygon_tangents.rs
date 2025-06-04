// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use super::{
    between_vectors_cached, cyclic_breadth_partition_search, dot_product, perp_dot_product,
};
use geo::Point;

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

    /// Calculates the tangents to the polygon exterior going through point `origin`.
    pub fn tangent_points(&self, origin: Point) -> Option<(I, I)> {
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
            cyclic_breadth_partition_search(0..len, |i: usize| {
                let prev = &poly_ext[(len + i - 1) % len];
                let cur = &poly_ext[i];
                let next = &poly_ext[(i + 1) % len];

                // local coordinate system with origin at `cur.0`.
                between_vectors_cached(cur.0 - origin, cur.0 - prev.0, cur.0 - next.0, cur.2)
            }) {
            ((pos_false + 1) % len, pos_true)
        } else if let (Some(mut rev_pos_false), Some(mut rev_pos_true)) =
            cyclic_breadth_partition_search(0..len, |i: usize| {
                let prev = &poly_ext[(len + i - 1) % len];
                let cur = &poly_ext[i];
                let next = &poly_ext[(i + 1) % len];

                // local coordinate system with origin at `cur.0`.
                between_vectors_cached(origin - cur.0, cur.0 - prev.0, cur.0 - next.0, cur.2)
            })
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

        Some((poly_ext[pos_true].1, poly_ext[pos_false].1))
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

#[cfg(test)]
mod tests {
    use super::poly_ext_tangent_points as petp;
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
}
