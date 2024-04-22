use geo::Point;
use petgraph::{
    data::FromElements,
    stable_graph::StableUnGraph,
    visit::{self, EdgeRef, NodeIndexable},
};
use spade::{HasPosition, InsertionError, Point2};

use crate::{
    drawing::{
        dot::FixedDotIndex,
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::MakePrimitiveShape,
        rules::RulesTrait,
    },
    geometry::primitive::PrimitiveShapeTrait,
    layout::Layout,
    triangulation::{GetVertexIndex, Triangulation, TriangulationEdgeWeight},
};

#[derive(Debug, Clone, Copy)]
pub struct VertexWeight {
    vertex: FixedDotIndex,
    pub pos: Point,
}

impl GetVertexIndex<FixedDotIndex> for VertexWeight {
    fn vertex_index(&self) -> FixedDotIndex {
        self.vertex
    }
}

impl HasPosition for VertexWeight {
    type Scalar = f64;
    fn position(&self) -> Point2<Self::Scalar> {
        Point2::new(self.pos.x(), self.pos.y())
    }
}

pub struct Ratsnest {
    graph: StableUnGraph<VertexWeight, TriangulationEdgeWeight, usize>,
}

impl Ratsnest {
    pub fn new(layout: &Layout<impl RulesTrait>) -> Result<Self, InsertionError> {
        let mut this = Self {
            graph: StableUnGraph::default(),
        };

        let mut triangulation =
            Triangulation::new(layout.drawing().geometry().graph().node_bound());

        for node in layout.drawing().primitive_nodes() {
            let center = node.primitive(layout.drawing()).shape().center();

            match node {
                PrimitiveIndex::FixedDot(dot) => {
                    triangulation.add_vertex(VertexWeight {
                        vertex: dot,
                        pos: center,
                    })?;
                }
                _ => (),
            }
        }

        this.graph =
            StableUnGraph::from_elements(petgraph::algo::min_spanning_tree(&triangulation));

        Ok(this)
    }

    pub fn graph(&self) -> &StableUnGraph<VertexWeight, TriangulationEdgeWeight, usize> {
        &self.graph
    }
}
