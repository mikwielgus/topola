use geo::{CoordsIter, Point, Polygon};
use rstar::AABB;
use topola::{
    geometry::primitive::{AccessPrimitiveShape, PrimitiveShape},
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
            PrimitiveShape::Bend(bend) => {
                let circle = bend.circle();

                let angle_from = bend.start_angle();
                let angle_step = bend.spanned_angle() / 100.0;

                let mut points: Vec<egui::Pos2> = vec![];

                for i in 0..=100 {
                    let x = circle.pos.x() + circle.r * (angle_from + i as f64 * angle_step).cos();
                    let y = circle.pos.y() + circle.r * (angle_from + i as f64 * angle_step).sin();
                    points.push(self.transform.mul_pos([x as f32, -y as f32].into()));
                }

                egui::Shape::line(
                    points,
                    egui::Stroke::new(bend.width as f32 * self.transform.scaling, color),
                )
            }
        };

        self.ui.painter().add(epaint_shape);

        if self.paint_bboxes {
            self.paint_bbox(AccessPrimitiveShape::bbox(shape, 0.0));
        }
    }

    pub fn paint_bbox(&mut self, bbox: AABB<[f64; 2]>) {
        let rect = egui::epaint::Rect {
            min: [bbox.lower()[0] as f32, -bbox.upper()[1] as f32].into(),
            max: [bbox.upper()[0] as f32, -bbox.lower()[1] as f32].into(),
        };
        self.ui.painter().add(egui::Shape::rect_stroke(
            self.transform * rect,
            egui::Rounding::ZERO,
            egui::Stroke::new(1.0, egui::Color32::GRAY),
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
}
