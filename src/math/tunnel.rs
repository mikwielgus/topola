// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT
//! Utilities for working with "tunnel vision"
//! (basically a simple kind of 2D ray tracing, where we are only interested
//! in incremental restriction / intersection of circle segments)

use crate::math::{between_vectors_cached, perp_dot_product};
use geo::Point;

#[derive(Clone, Copy, Debug)]
/// circle segment given as offsets from the origin
/// (not necessarily with the same radius, only the angle matters)
/// oriented counter-clockwise
pub struct Tunnel(pub Point, pub Point);

impl Tunnel {
    fn cross(&self) -> f64 {
        perp_dot_product(self.0, self.1)
    }

    fn between_vectors(&self, cross: f64, p: Point) -> bool {
        between_vectors_cached(p, self.0, self.1, cross)
    }

    pub fn intersection(self, othr: &Self) -> Option<Self> {
        let cross_self = self.cross();
        let cross_othr = othr.cross();

        // update segment data
        let in_between = |p_self: Point, p_othr: Point| {
            if p_self == p_othr {
                Some(p_self)
            } else if self.between_vectors(cross_self, p_othr) {
                Some(p_othr)
            } else if othr.between_vectors(cross_othr, p_self) {
                Some(p_self)
            } else {
                None
            }
        };

        let lhs = in_between(self.0, othr.0)?;
        let rhs = in_between(self.1, othr.1)?;

        if lhs == rhs {
            None
        } else {
            Some(Self(lhs, rhs))
        }
    }
}
