// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::{geometry::Point, Line};
use specctra_core::math::Circle;
use thiserror::Error;

use super::{seq_perp_dot_product, LineInGeneralForm, RotationSense};

#[derive(Error, Debug, Clone, Copy, PartialEq)]
#[error("no tangents for {0:?} and {1:?}")] // TODO add real error message
pub struct NoBitangents(pub Circle, pub Circle);

fn _bitangent(center: Point, r1: f64, r2: f64) -> Result<LineInGeneralForm, ()> {
    // Taken from https://cp-algorithms.com/geometry/tangents-to-two-circles.html
    // with small changes.

    if approx::relative_eq!(center.x(), 0.0) && approx::relative_eq!(center.y(), 0.0) {
        return Err(());
    }

    let epsilon = 1e-9;
    let dr = r2 - r1;
    let norm = center.x() * center.x() + center.y() * center.y();
    let discriminant = norm - dr * dr;

    if discriminant < -epsilon {
        return Err(());
    }

    let sqrt_discriminant = f64::sqrt(f64::abs(discriminant));

    Ok(LineInGeneralForm {
        a: (center.x() * dr + center.y() * sqrt_discriminant) / norm,
        b: (center.y() * dr - center.x() * sqrt_discriminant) / norm,
        c: r1,
    })
}

fn _bitangents(circle1: Circle, circle2: Circle) -> Vec<LineInGeneralForm> {
    let mut tgs: Vec<LineInGeneralForm> = [
        _bitangent((circle2 - circle1).pos, -circle1.r, -circle2.r),
        _bitangent((circle2 - circle1).pos, -circle1.r, circle2.r),
        _bitangent((circle2 - circle1).pos, circle1.r, -circle2.r),
        _bitangent((circle2 - circle1).pos, circle1.r, circle2.r),
    ]
    .into_iter()
    .flatten()
    .collect();

    for tg in tgs.iter_mut() {
        tg.c -= tg.a * circle1.pos.x() + tg.b * circle1.pos.y();
    }

    tgs
}

fn cast_point_to_line(pt: Point, line: LineInGeneralForm) -> Point {
    (
        (line.b * (line.b * pt.x() - line.a * pt.y()) - line.a * line.c)
            / (line.a * line.a + line.b * line.b),
        (line.a * (-line.b * pt.x() + line.a * pt.y()) - line.b * line.c)
            / (line.a * line.a + line.b * line.b),
    )
        .into()
}

fn bitangent_point_pairs(
    circle1: Circle,
    circle2: Circle,
) -> Result<Vec<(Point, Point)>, NoBitangents> {
    let point_pairs: Vec<(Point, Point)> = _bitangents(circle1, circle2)
        .into_iter()
        .map(|tg| {
            (
                cast_point_to_line(circle1.pos, tg),
                cast_point_to_line(circle2.pos, tg),
            )
        })
        .collect();

    if point_pairs.is_empty() {
        return Err(NoBitangents(circle1, circle2));
    }
    Ok(point_pairs)
}

pub fn bitangents(
    circle1: Circle,
    maybe_sense1: Option<RotationSense>,
    circle2: Circle,
    maybe_sense2: Option<RotationSense>,
) -> Result<impl Iterator<Item = Line>, NoBitangents> {
    Ok(bitangent_point_pairs(circle1, circle2)?
        .into_iter()
        .filter_map(move |tangent_point_pair| {
            if let Some(sense1) = maybe_sense1 {
                let cross1 =
                    seq_perp_dot_product(tangent_point_pair.0, tangent_point_pair.1, circle1.pos);

                if (sense1 == RotationSense::Clockwise && cross1 <= 0.0)
                    || (sense1 == RotationSense::Counterclockwise && cross1 >= 0.0)
                {
                    return None;
                }
            }

            if let Some(sense2) = maybe_sense2 {
                let cross2 =
                    seq_perp_dot_product(tangent_point_pair.0, tangent_point_pair.1, circle2.pos);

                if (sense2 == RotationSense::Clockwise && cross2 >= 0.0)
                    || (sense2 == RotationSense::Counterclockwise && cross2 <= 0.0)
                {
                    return None;
                }
            }

            Some(Line::new(tangent_point_pair.0, tangent_point_pair.1))
        }))
}

pub fn bitangent(
    circle1: Circle,
    maybe_sense1: Option<RotationSense>,
    circle2: Circle,
    maybe_sense2: Option<RotationSense>,
) -> Result<Line, NoBitangents> {
    Ok(bitangents(circle1, maybe_sense1, circle2, maybe_sense2)?
        .next()
        .ok_or(NoBitangents(circle1, circle2))?)
}
