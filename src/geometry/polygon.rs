use enum_dispatch::enum_dispatch;

pub struct PolygonShape {
    pub polygon: Polygon,
}

impl ShapeTrait for PolygonShape {
    fn contains_point(&self, p: Point) -> bool {
        self.polygon.contains(p)
    }
}
