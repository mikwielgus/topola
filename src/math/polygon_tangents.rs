// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use super::{between_vectors, cyclic_breadth_partition_search};
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

/// Calculates the tangents to the polygon exterior `poly_ext` oriented `cw?=poly_ext_is_cw`
/// going through point `origin`.
pub fn poly_ext_tangent_points<I: Copy>(
    poly_ext: &[(Point, I)],
    poly_ext_is_cw: bool,
    origin: Point,
) -> Result<(I, I), PolyTangentException<I>> {
    if poly_ext.is_empty() {
        return Err(PolyTangentException::EmptyTargetPolygon { origin });
    }

    let (pos_false, pos_true) = cyclic_breadth_partition_search(0..poly_ext.len(), &|i: usize| {
        let prev = &poly_ext[(poly_ext.len() + i - 1) % poly_ext.len()];
        let cur = &poly_ext[i];
        let next = &poly_ext[(i + 1) % poly_ext.len()];

        // local coordinate system with origin at `cur.0`.
        between_vectors(cur.0 - origin, cur.0 - prev.0, cur.0 - next.0)
    });

    let (mut pos_false, mut pos_true) =
        if let (Some(pos_false), Some(pos_true)) = (pos_false, pos_true) {
            (pos_false, pos_true)
        } else {
            return Err(PolyTangentException::InvalidData {
                poly_ext: poly_ext.to_vec().into_boxed_slice(),
                origin,
            });
        };

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

    // TODO: can we (without too much overhead) determine if `poly_ext` is CW or CCW?

    if poly_ext_is_cw {
        pos_true += 1;
        pos_true %= poly_ext.len();
    } else {
        pos_false += 1;
        pos_false %= poly_ext.len();
    }

    Ok((poly_ext[pos_true].1, poly_ext[pos_false].1))
}

#[cfg(test)]
mod tests {
    use super::poly_ext_tangent_points as petp;
    use crate::drawing::dot::FixedDotIndex;
    use geo::point;

    #[test]
    fn petp00() {
        let poly_ext = &[
            (point! { x: 0.0, y: 0.0 }, FixedDotIndex::new(0.into())),
            (point! { x: 1.0, y: 0.0 }, FixedDotIndex::new(1.into())),
            (point! { x: 1.0, y: 1.0 }, FixedDotIndex::new(2.into())),
            (point! { x: 0.0, y: 1.0 }, FixedDotIndex::new(3.into())),
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
            (point! { x: 0.0, y: 0.0 }, FixedDotIndex::new(0.into())),
            (point! { x: 0.0, y: 1.0 }, FixedDotIndex::new(3.into())),
            (point! { x: 1.0, y: 1.0 }, FixedDotIndex::new(2.into())),
            (point! { x: 1.0, y: 0.0 }, FixedDotIndex::new(1.into())),
        ];
        let origin = point! { x: 0.5, y: -1.0 };
        assert_eq!(
            petp(poly_ext, true, origin),
            Ok((FixedDotIndex::new(1.into()), FixedDotIndex::new(0.into())))
        );
    }
}
