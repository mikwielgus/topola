// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::{geometry::Point, Line};
use specctra_core::math::Circle;
use thiserror::Error;

use super::seq_perp_dot_product;

#[derive(Error, Debug, Clone, Copy, PartialEq)]
#[error("no tangents for {0:?} and {1:?}")] // TODO add real error message
pub struct NoTangents(pub Circle, pub Circle);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanonicalLine {
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

fn _tangent(center: Point, r1: f64, r2: f64) -> Result<CanonicalLine, ()> {
    let epsilon = 1e-9;
    let dr = r2 - r1;
    let norm = center.x() * center.x() + center.y() * center.y();
    let discriminant = norm - dr * dr;

    if discriminant < -epsilon {
        return Err(());
    }

    let sqrt_discriminant = f64::sqrt(f64::abs(discriminant));

    Ok(CanonicalLine {
        a: (center.x() * dr + center.y() * sqrt_discriminant) / norm,
        b: (center.y() * dr - center.x() * sqrt_discriminant) / norm,
        c: r1,
    })
}

fn _tangents(circle1: Circle, circle2: Circle) -> Result<[CanonicalLine; 4], ()> {
    let mut tgs: [CanonicalLine; 4] = [
        _tangent((circle2 - circle1).pos, -circle1.r, -circle2.r)?,
        _tangent((circle2 - circle1).pos, -circle1.r, circle2.r)?,
        _tangent((circle2 - circle1).pos, circle1.r, -circle2.r)?,
        _tangent((circle2 - circle1).pos, circle1.r, circle2.r)?,
    ];

    for tg in tgs.iter_mut() {
        tg.c -= tg.a * circle1.pos.x() + tg.b * circle1.pos.y();
    }

    Ok(tgs)
}

fn cast_point_to_canonical_line(pt: Point, line: CanonicalLine) -> Point {
    (
        (line.b * (line.b * pt.x() - line.a * pt.y()) - line.a * line.c)
            / (line.a * line.a + line.b * line.b),
        (line.a * (-line.b * pt.x() + line.a * pt.y()) - line.b * line.c)
            / (line.a * line.a + line.b * line.b),
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
    cw1: Option<bool>,
    circle2: Circle,
    cw2: Option<bool>,
) -> Result<impl Iterator<Item = Line>, NoTangents> {
    Ok(tangent_point_pairs(circle1, circle2)?
        .into_iter()
        .filter_map(move |tangent_point_pair| {
            if let Some(cw1) = cw1 {
                let cross1 =
                    seq_perp_dot_product(tangent_point_pair.0, tangent_point_pair.1, circle1.pos);

                if (cw1 && cross1 <= 0.0) || (!cw1 && cross1 >= 0.0) {
                    return None;
                }
            }

            if let Some(cw2) = cw2 {
                let cross2 =
                    seq_perp_dot_product(tangent_point_pair.0, tangent_point_pair.1, circle2.pos);

                if (cw2 && cross2 >= 0.0) || (!cw2 && cross2 <= 0.0) {
                    return None;
                }
            }

            Some(Line::new(tangent_point_pair.0, tangent_point_pair.1))
        }))
}

pub fn tangent_segment(
    circle1: Circle,
    cw1: Option<bool>,
    circle2: Circle,
    cw2: Option<bool>,
) -> Result<Line, NoTangents> {
    Ok(tangent_segments(circle1, cw1, circle2, cw2)?
        .next()
        .unwrap())
}
