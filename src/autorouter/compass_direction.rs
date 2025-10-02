// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use geo::Point;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompassDirection8 {
    North,
    NorthWest,
    West,
    SouthWest,
    South,
    SouthEast,
    East,
    NorthEast,
}

impl From<CompassDirection8> for Point {
    fn from(compass_direction8: CompassDirection8) -> Point {
        match compass_direction8 {
            CompassDirection8::North => [0.0, -1.0].into(),
            CompassDirection8::NorthWest => [-1.0, -1.0].into(),
            CompassDirection8::West => [-1.0, 0.0].into(),
            CompassDirection8::SouthWest => [-1.0, 1.0].into(),
            CompassDirection8::South => [0.0, 1.0].into(),
            CompassDirection8::SouthEast => [1.0, 1.0].into(),
            CompassDirection8::East => [1.0, 0.0].into(),
            CompassDirection8::NorthEast => [1.0, -1.0].into(),
        }
    }
}

impl CompassDirection8 {
    pub fn nearest_to_vector(vector: Point) -> Self {
        /*match (vector.x().signum(), vector.y().signum()) {
            (0.0, -1.0) => Self::North,
            (-1.0, -1.0) => Self::NorthWest,
            (-1.0, 0.0) => Self::West,
            (-1.0, 1.0) => Self::SouthWest,
            (0.0, 1.0) => Self::South,
            (1.0, 1.0) => Self::SouthEast,
            (1.0, 0.0) => Self::East,
            (1.0, -1.0) => Self::NorthEast,
            (_, _) => panic!(),
        }*/
        if vector.x() == 0.0 && vector.y() == 0.0 {
            panic!("Zero vector has no direction");
        }

        let angle = vector.y().atan2(vector.x()); // atan2 gives angle in radians from -π to π
        let angle_deg = angle.to_degrees(); // Convert to degrees for easier reasoning
        let compass_angle = (450.0 - angle_deg) % 360.0; // Convert to compass heading (0° = North)

        // Each direction is 45 degrees wide; round to nearest
        match ((compass_angle + 22.5) / 45.0).floor() as i32 % 8 {
            0 => CompassDirection8::North,
            1 => CompassDirection8::NorthEast,
            2 => CompassDirection8::East,
            3 => CompassDirection8::SouthEast,
            4 => CompassDirection8::South,
            5 => CompassDirection8::SouthWest,
            6 => CompassDirection8::West,
            7 => CompassDirection8::NorthWest,
            _ => unreachable!(),
        }
    }

    pub fn turn_counterclockwise(self) -> Self {
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

    pub fn turn_clockwise(self) -> Self {
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
}
