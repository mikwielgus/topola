use geo::{CoordsIter, Point, Polygon};
use pathfinder_canvas::{
    vec2f, ArcDirection, Canvas, CanvasRenderingContext2D, ColorU, FillRule, Path2D, RectF,
};
use topola::geometry::primitive::{PrimitiveShape, PrimitiveShapeTrait};

pub struct Painter<'a> {
    canvas: &'a mut CanvasRenderingContext2D,
}

impl<'a> Painter<'a> {
    pub fn new(canvas: &'a mut CanvasRenderingContext2D) -> Self {
        Self { canvas }
    }

    pub fn paint_primitive(&mut self, shape: &PrimitiveShape, color: ColorU, zoom: f32) {
        self.canvas.set_stroke_style(color);
        self.canvas.set_fill_style(color);

        match shape {
            PrimitiveShape::Dot(dot) => {
                let mut path = Path2D::new();
                path.ellipse(
                    vec2f(dot.c.pos.x() as f32, -dot.c.pos.y() as f32),
                    dot.c.r as f32,
                    0.0,
                    0.0,
                    std::f32::consts::TAU,
                );
                self.canvas.fill_path(path, FillRule::Winding);
            }
            PrimitiveShape::Seg(seg) => {
                let mut path = Path2D::new();
                path.move_to(vec2f(seg.from.x() as f32, -seg.from.y() as f32));
                path.line_to(vec2f(seg.to.x() as f32, -seg.to.y() as f32));
                self.canvas.set_line_width(seg.width as f32);
                self.canvas.stroke_path(path);
            }
            PrimitiveShape::Bend(bend) => {
                /*let delta1 = bend.from - bend.c.pos;
                let delta2 = bend.to - bend.c.pos;

                let angle1 = delta1.y().atan2(delta1.x());
                let angle2 = delta2.y().atan2(delta2.x());

                let mut path = Path2D::new();
                path.arc(
                    vec2f(bend.c.pos.x() as f32, -bend.c.pos.y() as f32),
                    bend.circle().r as f32,
                    angle1 as f32,
                    angle2 as f32,
                    ArcDirection::CW,
                );
                self.canvas.set_line_width(bend.width as f32);
                self.canvas.stroke_path(path);*/
            }
        }

        let envelope = PrimitiveShapeTrait::envelope(shape, 0.0);
        // XXX: points represented as arrays can't be conveniently converted to vector types
        let topleft = vec2f(envelope.lower()[0] as f32, -envelope.upper()[1] as f32);
        let bottomright = vec2f(envelope.upper()[0] as f32, -envelope.lower()[1] as f32);
        self.canvas.set_line_width(2.0 / zoom);
        self.canvas
            .set_stroke_style(ColorU::new(100, 100, 100, 255));
        self.canvas
            .stroke_rect(RectF::new(topleft, bottomright - topleft));
    }

    pub fn paint_polygon(&mut self, polygon: &Polygon, color: ColorU, zoom: f32) {
        let mut path = Path2D::new();
        let mut it = polygon.exterior_coords_iter();

        if let Some(initial_vertex) = it.next() {
            path.move_to(vec2f(initial_vertex.x as f32, -initial_vertex.y as f32));
        }

        for vertex in it {
            path.line_to(vec2f(vertex.x as f32, -vertex.y as f32));
        }

        path.close_path();

        self.canvas.set_stroke_style(color);
        self.canvas.set_fill_style(color);
        self.canvas.set_line_width(0.0);
        self.canvas.fill_path(path, FillRule::Winding);
    }

    pub fn paint_edge(&mut self, from: Point, to: Point, color: ColorU, zoom: f32) {
        let mut path = Path2D::new();
        path.move_to(vec2f(from.x() as f32, from.y() as f32));
        path.line_to(vec2f(to.x() as f32, to.y() as f32));
        self.canvas.set_stroke_style(color);
        self.canvas.set_line_width(2.0 / zoom);
        self.canvas.stroke_path(path);
    }
}
