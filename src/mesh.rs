use enum_dispatch::enum_dispatch;
use geo::Point;
use petgraph::visit;
use petgraph::{stable_graph::NodeIndex, visit::EdgeRef};
use spade::{HasPosition, InsertionError, Point2};

use crate::{
    graph::{FixedBendIndex, FixedDotIndex, GetNodeIndex, Index, LooseBendIndex, MakePrimitive},
    layout::Layout,
    primitive::MakeShape,
    shape::ShapeTrait,
    triangulation::{GetVertexIndex, Triangulation},
};

#[enum_dispatch(GetNodeIndex)]
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum VertexIndex {
    FixedDot(FixedDotIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

#[derive(Debug, Clone)]
struct TriangulationWeight {
    vertex: VertexIndex,
    pos: Point,
}

impl GetVertexIndex<VertexIndex> for TriangulationWeight {
    fn vertex(&self) -> VertexIndex {
        self.vertex
    }
}

impl HasPosition for TriangulationWeight {
    type Scalar = f64;
    fn position(&self) -> Point2<Self::Scalar> {
        Point2::new(self.pos.x(), self.pos.y())
    }
}

#[derive(Debug, Clone)]
pub struct Mesh {
    triangulation: Triangulation<VertexIndex, TriangulationWeight>,
}

impl Mesh {
    pub fn new(layout: &Layout) -> Self {
        Self {
            triangulation: Triangulation::new(layout),
        }
    }

    pub fn generate(&mut self, layout: &Layout) -> Result<(), InsertionError> {
        for node in layout.nodes() {
            let center = node.primitive(&layout.graph).shape().center();

            match node {
                Index::FixedDot(fixed_dot) => {
                    self.triangulation.add_vertex(TriangulationWeight {
                        vertex: fixed_dot.into(),
                        pos: center,
                    })?;
                }
                Index::FixedBend(fixed_bend) => {
                    self.triangulation.add_vertex(TriangulationWeight {
                        vertex: fixed_bend.into(),
                        pos: center,
                    })?;
                }
                /*Index::LooseBend(loose_bend) => {
                    self.triangulation.add_bend(
                        loose_bend.into(),
                        layout.primitive(loose_bend).core().into(),
                    );
                }*/
                _ => (),
            }
        }
        Ok(())
    }

    pub fn position(&self, vertex: VertexIndex) -> Point {
        self.triangulation.position(vertex)
    }
}

impl visit::GraphBase for Mesh {
    type NodeId = VertexIndex;
    type EdgeId = (VertexIndex, VertexIndex);
}

impl visit::Data for Mesh {
    type NodeWeight = ();
    type EdgeWeight = ();
}

#[derive(Debug, Clone, Copy)]
pub struct MeshEdgeReference {
    from: VertexIndex,
    to: VertexIndex,
}

impl visit::EdgeRef for MeshEdgeReference {
    type NodeId = VertexIndex;
    type EdgeId = (VertexIndex, VertexIndex);
    type Weight = ();

    fn source(&self) -> Self::NodeId {
        self.from
    }

    fn target(&self) -> Self::NodeId {
        self.to
    }

    fn weight(&self) -> &Self::Weight {
        &()
    }

    fn id(&self) -> Self::EdgeId {
        (self.from, self.to)
    }
}

impl<'a> visit::IntoEdgeReferences for &'a Mesh {
    type EdgeRef = MeshEdgeReference;
    type EdgeReferences = Box<dyn Iterator<Item = MeshEdgeReference> + 'a>;

    fn edge_references(self) -> Self::EdgeReferences {
        Box::new(
            self.triangulation
                .edge_references()
                .map(|edge| MeshEdgeReference {
                    from: edge.source(),
                    to: edge.target(),
                }),
        )
    }
}

impl<'a> visit::IntoNeighbors for &'a Mesh {
    type Neighbors = Box<dyn Iterator<Item = VertexIndex> + 'a>;

    fn neighbors(self, vertex: Self::NodeId) -> Self::Neighbors {
        self.triangulation.neighbors(vertex)
    }
}

impl<'a> visit::IntoEdges for &'a Mesh {
    type Edges = Box<dyn Iterator<Item = MeshEdgeReference> + 'a>;

    fn edges(self, node: Self::NodeId) -> Self::Edges {
        Box::new(
            self.triangulation
                .edges(node)
                .map(|edge| MeshEdgeReference {
                    from: edge.source(),
                    to: edge.target(),
                }),
        )
    }
}
