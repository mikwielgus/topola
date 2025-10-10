// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::Point;

pub trait CompassDirection: Copy + PartialEq + Into<Point> {
    fn nearest_from_vector(vector: Point) -> Self;
    fn turn_clockwise(self) -> Self;
    fn turn_counterclockwise(self) -> Self;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CardinalDirection {
    North,
    West,
    South,
    East,
}

impl From<CardinalDirection> for Point {
    fn from(cardinal_direction: CardinalDirection) -> Point {
        match cardinal_direction {
            CardinalDirection::North => [0.0, -1.0].into(),
            CardinalDirection::West => [-1.0, 0.0].into(),
            CardinalDirection::South => [0.0, 1.0].into(),
            CardinalDirection::East => [1.0, 0.0].into(),
        }
    }
}

impl CompassDirection for CardinalDirection {
    fn nearest_from_vector(vector: Point) -> Self {
        if vector.x().abs() > vector.y().abs() {
            if vector.x() > 0.0 {
                Self::East
            } else {
                Self::West
            }
        } else {
            if vector.y() > 0.0 {
                Self::South
            } else {
                Self::North
            }
        }
    }

    fn turn_clockwise(self) -> Self {
        match self {
            Self::North => Self::East,
            Self::East => Self::South,
            Self::South => Self::West,
            Self::West => Self::North,
        }
    }

    fn turn_counterclockwise(self) -> Self {
        match self {
            Self::North => Self::West,
            Self::West => Self::South,
            Self::South => Self::East,
            Self::East => Self::North,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrdinalDirection {
    NorthWest,
    SouthWest,
    SouthEast,
    NorthEast,
}

impl From<OrdinalDirection> for Point {
    fn from(ordinal_direction: OrdinalDirection) -> Point {
        match ordinal_direction {
            OrdinalDirection::NorthWest => [-1.0, -1.0].into(),
            OrdinalDirection::SouthWest => [-1.0, 1.0].into(),
            OrdinalDirection::SouthEast => [1.0, 1.0].into(),
            OrdinalDirection::NorthEast => [1.0, -1.0].into(),
        }
    }
}

impl CompassDirection for OrdinalDirection {
    fn nearest_from_vector(vector: Point) -> Self {
        if vector.x() > 0.0 && vector.y() > 0.0 {
            Self::SouthEast
        } else if vector.x() > 0.0 && vector.y() < 0.0 {
            Self::NorthEast
        } else if vector.x() < 0.0 && vector.y() < 0.0 {
            Self::NorthWest
        } else {
            Self::NorthEast
        }
    }

    fn turn_clockwise(self) -> Self {
        match self {
            Self::NorthWest => Self::SouthWest,
            Self::SouthWest => Self::SouthEast,
            Self::SouthEast => Self::SouthWest,
            Self::NorthEast => Self::NorthWest,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

impl From<PrincipalWind> for Point {
    fn from(principal_wind: PrincipalWind) -> Point {
        match principal_wind {
            PrincipalWind::North => [0.0, -1.0].into(),
            PrincipalWind::NorthWest => [-1.0, -1.0].into(),
            PrincipalWind::West => [-1.0, 0.0].into(),
            PrincipalWind::SouthWest => [-1.0, 1.0].into(),
            PrincipalWind::South => [0.0, 1.0].into(),
            PrincipalWind::SouthEast => [1.0, 1.0].into(),
            PrincipalWind::East => [1.0, 0.0].into(),
            PrincipalWind::NorthEast => [1.0, -1.0].into(),
        }
    }
}

impl CompassDirection for PrincipalWind {
    fn nearest_from_vector(vector: Point) -> Self {
        if vector.x() == 0.0 && vector.y() == 0.0 {
            panic!("Zero vector has no direction");
        }

        let angle = vector.y().atan2(vector.x()); // atan2 gives angle in radians from -pi to pi.
        let angle_deg = angle.to_degrees(); // Convert to degrees for easier reasoning.
        let compass_angle = (450.0 - angle_deg) % 360.0; // Convert to compass heading (0 deg = North).

        // Each direction is 45 degrees wide; round to nearest.
        match ((compass_angle + 22.5) / 45.0).floor() as i32 % 8 {
            0 => PrincipalWind::North,
            1 => PrincipalWind::NorthEast,
            2 => PrincipalWind::East,
            3 => PrincipalWind::SouthEast,
            4 => PrincipalWind::South,
            5 => PrincipalWind::SouthWest,
            6 => PrincipalWind::West,
            7 => PrincipalWind::NorthWest,
            _ => unreachable!(),
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
