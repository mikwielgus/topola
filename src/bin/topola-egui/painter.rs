use egui::{emath::RectTransform, epaint, Color32, Pos2, Ui};
use geo::Point;
use topola::layout::geometry::shape::Shape;

pub struct Painter<'a> {
    ui: &'a mut egui::Ui,
    transform: RectTransform,
}

impl<'a> Painter<'a> {
    pub fn new(ui: &'a mut Ui, transform: RectTransform) -> Self {
        Self { ui, transform }
    }

    pub fn paint_shape(&mut self, shape: &Shape, color: Color32) {
        let epaint_shape = match shape {
            Shape::Dot(dot) => epaint::Shape::circle_filled(
                self.transform
                    .transform_pos([dot.c.pos.x() as f32, dot.c.pos.y() as f32].into()),
                dot.c.r as f32,
                color,
            ),
            Shape::Seg(seg) => epaint::Shape::line_segment(
                [
                    self.transform
                        .transform_pos([seg.from.x() as f32, seg.from.y() as f32].into()),
                    self.transform
                        .transform_pos([seg.to.x() as f32, seg.to.y() as f32].into()),
                ],
                egui::Stroke::new(seg.width as f32, color),
            ),
            Shape::Bend(bend) => {
                let delta_from = bend.from - bend.c.pos;
                let delta_to = bend.to - bend.c.pos;

                let angle_from = delta_from.y().atan2(delta_from.x());
                let angle_to = delta_to.y().atan2(delta_to.x());
                let mut points: Vec<Pos2> = vec![];

                let angle_step = (angle_to - angle_from) / 100.0;

                for i in 0..100 {
                    let x = bend.c.pos.x() + bend.c.r * (angle_from + i as f64 * angle_step).cos();
                    let y = bend.c.pos.y() + bend.c.r * (angle_from + i as f64 * angle_step).sin();
                    points.push(self.transform.transform_pos([x as f32, y as f32].into()));
                }

                epaint::Shape::line(points, egui::Stroke::new(bend.width as f32, color))
            }
        };

        self.ui.painter().add(epaint_shape);
    }

    pub fn paint_edge(&mut self, from: Point, to: Point, color: Color32) {
        //
    }
}
