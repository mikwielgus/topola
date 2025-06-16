// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::{CoordsIter, LineString, Point, Polygon};
use rstar::AABB;
use topola::{
    geometry::{primitive::PrimitiveShape, shape::AccessShape},
    math::Circle,
};

pub struct Painter<'a> {
    ui: &'a mut egui::Ui,
    transform: egui::emath::TSTransform,
    paint_bboxes: bool,
}

impl<'a> Painter<'a> {
    pub fn new(
        ui: &'a mut egui::Ui,
        transform: egui::emath::TSTransform,
        paint_bboxes: bool,
    ) -> Self {
        Self {
            ui,
            transform,
            paint_bboxes,
        }
    }

    pub fn paint_primitive(&mut self, shape: &PrimitiveShape, color: egui::epaint::Color32) {
        let epaint_shape = match shape {
            PrimitiveShape::Dot(dot) => self.dot_shape(dot.circle, color),
            PrimitiveShape::Seg(seg) => egui::Shape::line_segment(
                [
                    self.transform
                        .mul_pos([seg.from.x() as f32, -seg.from.y() as f32].into()),
                    self.transform
                        .mul_pos([seg.to.x() as f32, -seg.to.y() as f32].into()),
                ],
                egui::Stroke::new(seg.width as f32 * self.transform.scaling, color),
            ),
            PrimitiveShape::Bend(bend) => egui::Shape::line(
                bend.render_discretization(101)
                    .map(|point| {
                        self.transform
                            .mul_pos([point.0.x as f32, -point.0.y as f32].into())
                    })
                    .collect(),
                egui::Stroke::new(bend.width as f32 * self.transform.scaling, color),
            ),
        };

        self.ui.painter().add(epaint_shape);

        if self.paint_bboxes {
            self.paint_bbox(AccessShape::bbox(shape, 0.0));
        }
    }

    pub fn paint_bbox(&mut self, bbox: AABB<[f64; 2]>) {
        self.paint_bbox_with_color(bbox, egui::Color32::GRAY)
    }

    pub fn paint_bbox_with_color(&mut self, bbox: AABB<[f64; 2]>, color: egui::Color32) {
        let rect = egui::epaint::Rect {
            min: [bbox.lower()[0] as f32, -bbox.upper()[1] as f32].into(),
            max: [bbox.upper()[0] as f32, -bbox.lower()[1] as f32].into(),
        };
        self.ui.painter().add(egui::Shape::rect_stroke(
            self.transform * rect,
            egui::CornerRadius::ZERO,
            egui::Stroke::new(1.0, color),
            egui::StrokeKind::Inside,
        ));
    }

    pub fn paint_dot(&mut self, circle: Circle, color: egui::epaint::Color32) {
        let shape = self.dot_shape(circle, color);
        self.ui.painter().add(shape);
    }

    fn dot_shape(&mut self, circle: Circle, color: egui::epaint::Color32) -> egui::Shape {
        egui::Shape::circle_filled(
            self.transform
                .mul_pos([circle.pos.x() as f32, -circle.pos.y() as f32].into()),
            circle.r as f32 * self.transform.scaling,
            color,
        )
    }

    pub fn paint_linestring(&mut self, linestring: &LineString, color: egui::epaint::Color32) {
        self.ui.painter().add(egui::Shape::line(
            linestring
                .exterior_coords_iter()
                .map(|coords| {
                    self.transform
                        .mul_pos([coords.x as f32, -coords.y as f32].into())
                })
                .collect(),
            egui::epaint::PathStroke {
                width: 5.0,
                color: egui::epaint::ColorMode::Solid(color),
                kind: egui::epaint::StrokeKind::Inside,
            },
        ));
    }

    pub fn paint_polygon(&mut self, polygon: &Polygon, color: egui::epaint::Color32) {
        self.ui.painter().add(egui::Shape::convex_polygon(
            polygon
                .exterior_coords_iter()
                .map(|coords| {
                    self.transform
                        .mul_pos([coords.x as f32, -coords.y as f32].into())
                })
                .collect(),
            color,
            egui::Stroke::default(),
        ));
    }

    pub fn paint_edge(&mut self, from: Point, to: Point, stroke: egui::Stroke) {
        self.ui.painter().add(egui::Shape::line_segment(
            [
                self.transform
                    .mul_pos([from.x() as f32, -from.y() as f32].into()),
                self.transform
                    .mul_pos([to.x() as f32, -to.y() as f32].into()),
            ],
            stroke,
        ));
    }

    pub fn paint_text(
        &mut self,
        pos: Point,
        anchor: egui::Align2,
        text: &str,
        color: egui::epaint::Color32,
    ) {
        let text = self.ui.painter().fonts(|fonts| {
            egui::Shape::text(
                fonts,
                self.transform
                    .mul_pos([pos.x() as f32, -pos.y() as f32].into()),
                anchor,
                text,
                egui::FontId::default(),
                color,
            )
        });
        self.ui.painter().add(text);
    }
}
