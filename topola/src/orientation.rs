// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::ops::Add;

use derive_more::From;
use num_traits::{Signed, Zero};
use serde::{Deserialize, Serialize};

use crate::{
    Vector2,
    compass::{CompassDirection, PrincipalWind},
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, From, Ord, PartialEq, PartialOrd, Serialize)]
pub enum OrthogonalOrientation {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, From, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Orientation {
    Horizontal,
    Vertical,
    Oblique,
}

impl Orientation {
    pub fn principal_wind<T>(self, vector: Vector2<T>) -> Option<PrincipalWind>
    where
        T: Copy + Add<Output = T> + PartialOrd + Signed + Zero,
    {
        match self {
            Self::Horizontal => {
                if vector.x.is_zero() {
                    None
                } else if vector.x > T::zero() {
                    Some(PrincipalWind::East)
                } else {
                    Some(PrincipalWind::West)
                }
            }
            Self::Vertical => {
                if vector.y.is_zero() {
                    None
                } else if vector.y < T::zero() {
                    Some(PrincipalWind::North)
                } else {
                    Some(PrincipalWind::South)
                }
            }
            Self::Oblique => {
                if vector.x.is_zero() && vector.y.is_zero() {
                    None
                } else {
                    Some(PrincipalWind::nearest_from_vector(vector))
                }
            }
        }
    }
}
