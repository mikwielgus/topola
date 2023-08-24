use petgraph::stable_graph::NodeIndex;
use spade::{DelaunayTriangulation, HasPosition, Point2, Triangulation};

use crate::{graph::DotIndex, layout::Layout, router::Router};

struct TriangulationVertex {
    pub index: DotIndex,
    x: f64,
    y: f64,
}

impl HasPosition for TriangulationVertex {
    type Scalar = f64;
    fn position(&self) -> Point2<Self::Scalar> {
        Point2::new(self.x, self.y)
    }
}

pub struct Mesh {
    triangulation: DelaunayTriangulation<TriangulationVertex>,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            triangulation: DelaunayTriangulation::new(),
        }
    }

    pub fn triangulate(&mut self, layout: &Layout) {
        self.triangulation.clear();

        for dot in layout.dots() {
            let center = layout.primitive(dot).shape().center();
            self.triangulation
                .insert(TriangulationVertex {
                    index: dot,
                    x: center.x(),
                    y: center.y(),
                })
                .unwrap(); // TODO.
        }
    }

    pub fn edges(&self) -> impl Iterator<Item = (DotIndex, DotIndex)> + '_ {
        self.triangulation
            .directed_edges()
            .map(|edge| (edge.from().as_ref().index, edge.to().as_ref().index))
    }
}
