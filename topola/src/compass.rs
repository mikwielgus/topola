// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::ops::{Add, Neg};

use num_traits::{One, Signed, Zero};
use serde::{Deserialize, Serialize};

use crate::Vector2;

pub trait CompassDirection<T>: Copy + PartialEq + Into<Vector2<T>> {
    fn nearest_from_vector(vector: Vector2<T>) -> Self;
    fn turn_clockwise(self) -> Self;
    fn turn_counterclockwise(self) -> Self;
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum CardinalDirection {
    East,
    North,
    West,
    South,
}

impl<T: Copy + Neg<Output = T> + One + Zero> From<CardinalDirection> for Vector2<T> {
    fn from(cardinal_direction: CardinalDirection) -> Vector2<T> {
        match cardinal_direction {
            CardinalDirection::East => [T::one(), T::zero()].into(),
            CardinalDirection::North => [T::zero(), -T::one()].into(),
            CardinalDirection::West => [-T::one(), T::zero()].into(),
            CardinalDirection::South => [T::zero(), T::one()].into(),
        }
    }
}

impl<T: Copy + PartialOrd + Signed + Zero> CompassDirection<T> for CardinalDirection {
    fn nearest_from_vector(vector: Vector2<T>) -> Self {
        if vector.x.abs() > vector.y.abs() {
            if vector.x > T::zero() {
                Self::East
            } else {
                Self::West
            }
        } else if vector.y > T::zero() {
            Self::South
        } else {
            Self::North
        }
    }

    fn turn_counterclockwise(self) -> Self {
        match self {
            Self::East => Self::North,
            Self::North => Self::West,
            Self::West => Self::South,
            Self::South => Self::East,
        }
    }

    fn turn_clockwise(self) -> Self {
        match self {
            Self::East => Self::South,
            Self::North => Self::East,
            Self::West => Self::North,
            Self::South => Self::West,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum OrdinalDirection {
    NorthWest,
    SouthWest,
    SouthEast,
    NorthEast,
}

impl<T: Copy + Neg<Output = T> + One + Zero> From<OrdinalDirection> for Vector2<T> {
    fn from(ordinal_direction: OrdinalDirection) -> Vector2<T> {
        match ordinal_direction {
            OrdinalDirection::NorthEast => [T::one(), -T::one()].into(),
            OrdinalDirection::NorthWest => [-T::one(), -T::one()].into(),
            OrdinalDirection::SouthWest => [-T::one(), T::one()].into(),
            OrdinalDirection::SouthEast => [T::one(), T::one()].into(),
        }
    }
}

impl<T: Copy + PartialOrd + Signed + Zero> CompassDirection<T> for OrdinalDirection {
    fn nearest_from_vector(vector: Vector2<T>) -> Self {
        let zero = T::zero();

        if vector.x > zero && vector.y > zero {
            Self::SouthEast
        } else if vector.x > zero && vector.y < zero {
            Self::NorthEast
        } else if vector.x < zero && vector.y < zero {
            Self::NorthWest
        } else {
            Self::SouthWest
        }
    }

    fn turn_clockwise(self) -> Self {
        match self {
            Self::NorthWest => Self::NorthEast,
            Self::NorthEast => Self::SouthEast,
            Self::SouthEast => Self::SouthWest,
            Self::SouthWest => Self::NorthWest,
        }
    }

    fn turn_counterclockwise(self) -> Self {
        match self {
            Self::NorthWest => Self::SouthWest,
            Self::SouthWest => Self::SouthEast,
            Self::SouthEast => Self::NorthEast,
            Self::NorthEast => Self::NorthWest,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PrincipalWind {
    North,
    NorthWest,
    West,
    SouthWest,
    South,
    SouthEast,
    East,
    NorthEast,
}

impl<T: Copy + Neg<Output = T> + One + Zero> From<PrincipalWind> for Vector2<T> {
    fn from(principal_wind: PrincipalWind) -> Vector2<T> {
        match principal_wind {
            PrincipalWind::North => [T::zero(), -T::one()].into(),
            PrincipalWind::NorthWest => [-T::one(), -T::one()].into(),
            PrincipalWind::West => [-T::one(), T::zero()].into(),
            PrincipalWind::SouthWest => [-T::one(), T::one()].into(),
            PrincipalWind::South => [T::zero(), T::one()].into(),
            PrincipalWind::SouthEast => [T::one(), T::one()].into(),
            PrincipalWind::East => [T::one(), T::zero()].into(),
            PrincipalWind::NorthEast => [T::one(), -T::one()].into(),
        }
    }
}

impl<T: Copy + Add<Output = T> + PartialOrd + Signed + Zero> CompassDirection<T> for PrincipalWind {
    fn nearest_from_vector(vector: Vector2<T>) -> Self {
        if vector.x.is_zero() && vector.y.is_zero() {
            panic!("zero vector has no direction");
        }

        if vector.x.is_zero() {
            return if vector.y < T::zero() {
                Self::North
            } else {
                Self::South
            };
        }

        if vector.y.is_zero() {
            return if vector.x > T::zero() {
                Self::East
            } else {
                Self::West
            };
        }

        if vector.x > T::zero() && vector.y < T::zero() {
            if vector.x.abs() > vector.y.abs() {
                if vector.y.abs() + vector.y.abs() <= vector.x.abs() {
                    Self::East
                } else {
                    Self::NorthEast
                }
            } else if vector.y.abs() > vector.x.abs() {
                if vector.x.abs() + vector.x.abs() <= vector.y.abs() {
                    Self::North
                } else {
                    Self::NorthEast
                }
            } else {
                Self::NorthEast
            }
        } else if vector.x > T::zero() && vector.y > T::zero() {
            if vector.x.abs() > vector.y.abs() {
                if vector.y.abs() + vector.y.abs() <= vector.x.abs() {
                    Self::East
                } else {
                    Self::SouthEast
                }
            } else if vector.y.abs() > vector.x.abs() {
                if vector.x.abs() + vector.x.abs() <= vector.y.abs() {
                    Self::South
                } else {
                    Self::SouthEast
                }
            } else {
                Self::SouthEast
            }
        } else if vector.x < T::zero() && vector.y > T::zero() {
            if vector.x.abs() > vector.y.abs() {
                if vector.y.abs() + vector.y.abs() <= vector.x.abs() {
                    Self::West
                } else {
                    Self::SouthWest
                }
            } else if vector.y.abs() > vector.x.abs() {
                if vector.x.abs() + vector.x.abs() <= vector.y.abs() {
                    Self::South
                } else {
                    Self::SouthWest
                }
            } else {
                Self::SouthWest
            }
        } else {
            if vector.x.abs() > vector.y.abs() {
                if vector.y.abs() + vector.y.abs() <= vector.x.abs() {
                    Self::West
                } else {
                    Self::NorthWest
                }
            } else if vector.y.abs() > vector.x.abs() {
                if vector.x.abs() + vector.x.abs() <= vector.y.abs() {
                    Self::North
                } else {
                    Self::NorthWest
                }
            } else {
                Self::NorthWest
            }
        }
    }

    fn turn_clockwise(self) -> Self {
        match self {
            Self::North => Self::NorthEast,
            Self::NorthEast => Self::East,
            Self::East => Self::SouthEast,
            Self::SouthEast => Self::South,
            Self::South => Self::SouthWest,
            Self::SouthWest => Self::West,
            Self::West => Self::NorthWest,
            Self::NorthWest => Self::North,
        }
    }

    fn turn_counterclockwise(self) -> Self {
        match self {
            Self::North => Self::NorthWest,
            Self::NorthWest => Self::West,
            Self::West => Self::SouthWest,
            Self::SouthWest => Self::South,
            Self::South => Self::SouthEast,
            Self::SouthEast => Self::East,
            Self::East => Self::NorthEast,
            Self::NorthEast => Self::North,
        }
    }
}
