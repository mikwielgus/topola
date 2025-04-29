// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::{geometry::Point, Line};
use specctra_core::math::Circle;
use thiserror::Error;

use super::{seq_perp_dot_product, NormalLine, RotationSense};

#[derive(Error, Debug, Clone, Copy, PartialEq)]
#[error("no tangents for {0:?} and {1:?}")] // TODO add real error message
pub struct NoTangents(pub Circle, pub Circle);

fn _tangent(center: Point, r1: f64, r2: f64) -> Result<NormalLine, ()> {
    let epsilon = 1e-9;
    let dr = r2 - r1;
    let norm = center.x() * center.x() + center.y() * center.y();
    let discriminant = norm - dr * dr;

    if discriminant < -epsilon {
        return Err(());
    }

    let sqrt_discriminant = f64::sqrt(f64::abs(discriminant));

    Ok(NormalLine {
        x: (center.x() * dr + center.y() * sqrt_discriminant) / norm,
        y: (center.y() * dr - center.x() * sqrt_discriminant) / norm,
        offset: r1,
    })
}

fn _tangents(circle1: Circle, circle2: Circle) -> Result<[NormalLine; 4], ()> {
    let mut tgs: [NormalLine; 4] = [
        _tangent((circle2 - circle1).pos, -circle1.r, -circle2.r)?,
        _tangent((circle2 - circle1).pos, -circle1.r, circle2.r)?,
        _tangent((circle2 - circle1).pos, circle1.r, -circle2.r)?,
        _tangent((circle2 - circle1).pos, circle1.r, circle2.r)?,
    ];

    for tg in tgs.iter_mut() {
        tg.offset -= tg.x * circle1.pos.x() + tg.y * circle1.pos.y();
    }

    Ok(tgs)
}

fn cast_point_to_canonical_line(pt: Point, line: NormalLine) -> Point {
    (
        (line.y * (line.y * pt.x() - line.x * pt.y()) - line.x * line.offset)
            / (line.x * line.x + line.y * line.y),
        (line.x * (-line.y * pt.x() + line.x * pt.y()) - line.y * line.offset)
            / (line.x * line.x + line.y * line.y),
    )
        .into()
}

fn tangent_point_pairs(
    circle1: Circle,
    circle2: Circle,
) -> Result<[(Point, Point); 4], NoTangents> {
    let tgs = _tangents(circle1, circle2).map_err(|_| NoTangents(circle1, circle2))?;

    Ok([
        (
            cast_point_to_canonical_line(circle1.pos, tgs[0]),
            cast_point_to_canonical_line(circle2.pos, tgs[0]),
        ),
        (
            cast_point_to_canonical_line(circle1.pos, tgs[1]),
            cast_point_to_canonical_line(circle2.pos, tgs[1]),
        ),
        (
            cast_point_to_canonical_line(circle1.pos, tgs[2]),
            cast_point_to_canonical_line(circle2.pos, tgs[2]),
        ),
        (
            cast_point_to_canonical_line(circle1.pos, tgs[3]),
            cast_point_to_canonical_line(circle2.pos, tgs[3]),
        ),
    ])
}

pub fn tangent_segments(
    circle1: Circle,
    maybe_sense1: Option<RotationSense>,
    circle2: Circle,
    maybe_sense2: Option<RotationSense>,
) -> Result<impl Iterator<Item = Line>, NoTangents> {
    Ok(tangent_point_pairs(circle1, circle2)?
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

pub fn tangent_segment(
    circle1: Circle,
    maybe_sense1: Option<RotationSense>,
    circle2: Circle,
    maybe_sense2: Option<RotationSense>,
) -> Result<Line, NoTangents> {
    Ok(
        tangent_segments(circle1, maybe_sense1, circle2, maybe_sense2)?
            .next()
            .unwrap(),
    )
}
