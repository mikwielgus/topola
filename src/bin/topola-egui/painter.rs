use geo::{CoordsIter, Point, Polygon};
use topola::geometry::primitive::{PrimitiveShape, PrimitiveShapeTrait};

pub struct Painter<'a> {
    ui: &'a mut egui::Ui,
    transform: egui::emath::RectTransform,
}

impl<'a> Painter<'a> {
    pub fn new(ui: &'a mut egui::Ui, transform: egui::emath::RectTransform) -> Self {
        Self { ui, transform }
    }

    pub fn paint_primitive(&mut self, shape: &PrimitiveShape, color: egui::epaint::Color32) {
        let epaint_shape = match shape {
            PrimitiveShape::Dot(dot) => egui::Shape::circle_filled(
                self.transform
                    .transform_pos([dot.c.pos.x() as f32, -dot.c.pos.y() as f32].into()),
                dot.c.r as f32 * self.transform.scale().x,
                color,
            ),
            PrimitiveShape::Seg(seg) => egui::Shape::line_segment(
                [
                    self.transform
                        .transform_pos([seg.from.x() as f32, -seg.from.y() as f32].into()),
                    self.transform
                        .transform_pos([seg.to.x() as f32, -seg.to.y() as f32].into()),
                ],
                egui::Stroke::new(seg.width as f32 * self.transform.scale().x, color),
            ),
            PrimitiveShape::Bend(bend) => {
                let delta_from = bend.from - bend.c.pos;
                let delta_to = bend.to - bend.c.pos;

                let angle_from = delta_from.y().atan2(delta_from.x());
                let angle_to = delta_to.y().atan2(delta_to.x());
                let mut points: Vec<egui::Pos2> = vec![];

                let angle_step = (angle_to - angle_from) / 100.0;

                for i in 0..100 {
                    let x = bend.c.pos.x() + bend.c.r * (angle_from + i as f64 * angle_step).cos();
                    let y = bend.c.pos.y() + bend.c.r * (angle_from + i as f64 * angle_step).sin();
                    points.push(self.transform.transform_pos([x as f32, -y as f32].into()));
                }

                egui::Shape::line(
                    points,
                    egui::Stroke::new(bend.width as f32 * self.transform.scale().x, color),
                )
            }
        };

        self.ui.painter().add(epaint_shape);

        let envelope = PrimitiveShapeTrait::envelope(shape, 0.0);
        let rect = egui::epaint::Rect {
            min: [envelope.lower()[0] as f32, -envelope.upper()[1] as f32].into(),
            max: [envelope.upper()[0] as f32, -envelope.lower()[1] as f32].into(),
        };
        self.ui.painter().add(egui::Shape::rect_stroke(
            self.transform.transform_rect(rect),
            egui::Rounding::ZERO,
            egui::Stroke::new(1.0, egui::Color32::GRAY),
        ));
    }

    pub fn paint_polygon(&mut self, polygon: &Polygon, color: egui::epaint::Color32) {
        self.ui.painter().add(egui::Shape::convex_polygon(
            polygon
                .exterior_coords_iter()
                .map(|coords| {
                    self.transform
                        .transform_pos([coords.x as f32, -coords.y as f32].into())
                })
                .collect(),
            color,
            egui::Stroke::default(),
        ));
    }

    pub fn paint_edge(&mut self, from: Point, to: Point, color: egui::epaint::Color32) {
        self.ui.painter().add(egui::Shape::line_segment(
            [
                self.transform
                    .transform_pos([from.x() as f32, -from.y() as f32].into()),
                self.transform
                    .transform_pos([to.x() as f32, -to.y() as f32].into()),
            ],
            egui::Stroke::new(1.0, color),
        ));
    }
}
