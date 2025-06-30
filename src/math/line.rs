// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use super::{dot_product, perp_dot_product};
use geo::{point, Line, LineString, Point};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineIntersection {
    Empty,
    Overlapping,
    Point(Point),
}

/// A line in the normal form: `x0*y + y0*y + offset = 0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalLine {
    pub x: f64,
    pub y: f64,
    pub offset: f64,
}

impl From<Line> for NormalLine {
    fn from(l: Line) -> Self {
        // the normal vector is perpendicular to the line
        let normal = point! {
            x: l.dy(),
            y: -l.dx(),
        };
        Self {
            x: normal.0.x,
            y: normal.0.y,
            offset: -perp_dot_product(l.end.into(), l.start.into()),
        }
    }
}

impl NormalLine {
    pub fn evaluate_at(&self, pt: Point) -> f64 {
        self.x * pt.x() + self.y * pt.y() + self.offset
    }

    pub fn angle(&self) -> f64 {
        self.y.atan2(self.x)
    }

    pub fn make_normal_unit(&mut self) {
        let normal_len = self.y.hypot(self.x);
        if normal_len > (f64::EPSILON * 16.0) {
            self.x /= normal_len;
            self.y /= normal_len;
            self.offset /= normal_len;
        }
    }

    /// Calculate the intersection between two lines.
    pub fn intersects(&self, b: &Self) -> LineIntersection {
        const ALMOST_ZERO: f64 = f64::EPSILON * 16.0;
        let (mut a, mut b) = (*self, *b);
        let _ = (a.make_normal_unit(), b.make_normal_unit());
        let apt = geo::point! { x: a.x, y: a.y };
        let bpt = geo::point! { x: b.x, y: b.y };
        let det = perp_dot_product(apt, bpt);
        let rpx = b.y * a.offset - a.y * b.offset;
        let rpy = -b.x * a.offset + a.x * b.offset;

        if det.abs() > ALMOST_ZERO {
            LineIntersection::Point(geo::point! { x: rpx, y: rpy } / det)
        } else if rpx.abs() <= ALMOST_ZERO && rpy.abs() <= ALMOST_ZERO {
            LineIntersection::Overlapping
        } else {
            LineIntersection::Empty
        }
    }

    /// Project the point `pt` onto this line, and generate a new line which is orthogonal
    /// to `self`, and goes through `pt`.
    #[inline]
    pub fn orthogonal_through(&self, pt: &Point) -> Self {
        Self {
            // recover the original parallel vector
            x: -self.y,
            y: self.x,
            offset: self.x * pt.0.y - self.y * pt.0.x,
        }
    }

    pub fn segment_interval(&self, line: &Line) -> core::ops::RangeInclusive<f64> {
        // recover the original parallel vector
        let parv = geo::point! {
            x: -self.y,
            y: self.x,
        };
        dot_product(parv, line.start.into())..=dot_product(parv, line.end.into())
    }

    pub fn segment_interval_ordered(&self, line: &Line) -> core::ops::RangeInclusive<f64> {
        let ret = self.segment_interval(line);
        if ret.start() <= ret.end() {
            ret
        } else {
            *ret.end()..=*ret.start()
        }
    }
}

/// Returns `Some(p)` when `p` lies in the intersection of the given lines.
pub fn intersect_lines(line1: &Line, line2: &Line) -> Option<Point> {
    let nline1 = NormalLine::from(*line1);
    let nline2 = NormalLine::from(*line2);

    match nline1.intersects(&nline2) {
        LineIntersection::Empty | LineIntersection::Overlapping => None,
        LineIntersection::Point(pt) => {
            let parv1 = geo::point! {
                x: line1.dx(),
                y: line1.dy(),
            };
            let parv2 = geo::point! {
                x: line2.dx(),
                y: line2.dy(),
            };
            // the following is more numerically robust than a `Line::contains` check
            if nline1
                .segment_interval_ordered(line1)
                .contains(&dot_product(parv1, pt))
                && nline2
                    .segment_interval_ordered(line2)
                    .contains(&dot_product(parv2, pt))
            {
                Some(pt)
            } else {
                None
            }
        }
    }
}

/// Returns `Some(p)` when `p` lies in the intersection of a line and a ray
/// (line which is only bounded at one side, i.e. point + directon)
pub fn intersect_line_and_ray(line1: &Line, ray2: &Line) -> Option<Point> {
    let nline1 = NormalLine::from(*line1);
    let nray2 = NormalLine::from(*ray2);

    match nline1.intersects(&nray2) {
        LineIntersection::Empty | LineIntersection::Overlapping => None,
        LineIntersection::Point(pt) => {
            let parv1 = geo::point! {
                x: line1.dx(),
                y: line1.dy(),
            };
            let parv2 = geo::point! {
                x: ray2.dx(),
                y: ray2.dy(),
            };
            // the following is more numerically robust than a `Line::contains` check
            let is_match = if nline1
                .segment_interval_ordered(line1)
                .contains(&dot_product(parv1, pt))
            {
                let nray2interval = nray2.segment_interval(ray2);
                let parv2pt = dot_product(parv2, pt);
                if nray2interval.start() <= nray2interval.end() {
                    *nray2interval.start() <= parv2pt
                } else {
                    *nray2interval.start() >= parv2pt
                }
            } else {
                false
            };
            if is_match {
                Some(pt)
            } else {
                None
            }
        }
    }
}

/// Returns `Some(p)` when `p` lies in the intersection of a linestring and a ray
pub fn intersect_linestring_and_ray(linestring: &LineString, ray: &Line) -> Option<Point> {
    for line in linestring.lines() {
        if let Some(pt) = intersect_line_and_ray(&line, ray) {
            return Some(pt);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersect_line_and_line00() {
        assert_eq!(
            intersect_lines(
                &Line {
                    start: geo::coord! { x: -1., y: -1. },
                    end: geo::coord! { x: 1., y: 1. },
                },
                &Line {
                    start: geo::coord! { x: -1., y: 1. },
                    end: geo::coord! { x: 1., y: -1. },
                }
            ),
            Some(geo::point! { x: 0., y: 0. })
        );
        assert_eq!(
            intersect_lines(
                &Line {
                    start: geo::coord! { x: -1., y: -1. },
                    end: geo::coord! { x: 1., y: 1. },
                },
                &Line {
                    start: geo::coord! { x: -1., y: 1. },
                    end: geo::coord! { x: -0.5, y: 0.5 },
                }
            ),
            None
        );
    }

    #[test]
    fn intersect_line_and_ray00() {
        assert_eq!(
            intersect_line_and_ray(
                &Line {
                    start: geo::coord! { x: -1., y: -1. },
                    end: geo::coord! { x: 1., y: 1. },
                },
                &Line {
                    start: geo::coord! { x: -1., y: 1. },
                    end: geo::coord! { x: 1., y: -1. },
                }
            ),
            Some(geo::point! { x: 0., y: 0. })
        );
        assert_eq!(
            intersect_line_and_ray(
                &Line {
                    start: geo::coord! { x: -1., y: -1. },
                    end: geo::coord! { x: 1., y: 1. },
                },
                &Line {
                    start: geo::coord! { x: -1., y: 1. },
                    end: geo::coord! { x: -0.5, y: 0.5 },
                }
            ),
            Some(geo::point! { x: 0., y: 0. })
        );
    }

    #[test]
    fn intersect_line_and_ray01() {
        assert_eq!(
            intersect_line_and_ray(
                &Line {
                    start: geo::coord! { x: -1., y: -1. },
                    end: geo::coord! { x: 1., y: 1. },
                },
                &Line {
                    start: geo::coord! { x: -3., y: -1. },
                    end: geo::coord! { x: -1., y: 1. },
                }
            ),
            None
        );
    }

    #[test]
    fn intersect_line_and_ray02() {
        let pt = intersect_line_and_ray(
            &Line {
                start: geo::coord! { x: 140., y: -110. },
                end: geo::coord! { x: 160., y: -110. },
            },
            &Line {
                start: geo::coord! { x: 148., y: -106. },
                end: geo::coord! { x: 148., y: -109. },
            },
        )
        .unwrap();

        approx::assert_abs_diff_eq!(pt.x(), 148.);
        approx::assert_abs_diff_eq!(pt.y(), -110.);
    }
}
