use enum_dispatch::enum_dispatch;
use fixedbitset::FixedBitSet;
use geo::{point, Point};
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::{self, NodeIndexable};
use spade::{
    handles::FixedVertexHandle, DelaunayTriangulation, HasPosition, InsertionError, Point2,
    Triangulation,
};

use crate::{
    graph::{FixedBendIndex, FixedDotIndex, GetNodeIndex, Index, LooseBendIndex},
    layout::Layout,
};
use crate::{primitive::MakeShape, shape::ShapeTrait};

#[derive(Debug, Clone)]
struct Vertex {
    graph_index: VertexIndex,
    x: f64,
    y: f64,
}

#[enum_dispatch(GetNodeIndex)]
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum VertexIndex {
    FixedDot(FixedDotIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

impl HasPosition for Vertex {
    type Scalar = f64;
    fn position(&self) -> Point2<Self::Scalar> {
        Point2::new(self.x, self.y)
    }
}

#[derive(Debug, Clone)]
pub struct Mesh {
    triangulation: DelaunayTriangulation<Vertex>,
    vertex_to_handle: Vec<Option<FixedVertexHandle>>,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            triangulation: DelaunayTriangulation::new(),
            vertex_to_handle: Vec::new(),
        }
    }

    pub fn triangulate(&mut self, layout: &Layout) -> Result<(), InsertionError> {
        self.triangulation.clear();
        self.vertex_to_handle = Vec::new();
        self.vertex_to_handle
            .resize(layout.graph.node_bound(), None);

        for node in layout.nodes() {
            if let Index::FixedDot(dot) = node {
                let center = layout.primitive(dot).shape().center();

                self.vertex_to_handle[dot.node_index().index()] =
                    Some(self.triangulation.insert(Vertex {
                        graph_index: dot.into(),
                        x: center.x(),
                        y: center.y(),
                    })?);
            }
        }

        Ok(())
    }

    pub fn vertex(&self, handle: FixedVertexHandle) -> VertexIndex {
        self.triangulation.vertex(handle).as_ref().graph_index
    }

    pub fn handle(&self, graph_index: VertexIndex) -> FixedVertexHandle {
        self.vertex_to_handle[graph_index.node_index().index()].unwrap()
    }

    pub fn position(&self, vertex: VertexIndex) -> Point {
        let position = self.triangulation.vertex(self.handle(vertex)).position();
        point! {x: position.x, y: position.y}
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

#[derive(Clone, Copy)]
pub struct MeshEdgeReference {
    from: VertexIndex,
    to: VertexIndex,
}

impl<'a> visit::EdgeRef for MeshEdgeReference {
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
                .directed_edges()
                .map(|edge| MeshEdgeReference {
                    from: self.vertex(edge.from().fix()),
                    to: self.vertex(edge.to().fix()),
                }),
        )
    }
}

impl<'a> visit::IntoNeighbors for &'a Mesh {
    type Neighbors = Box<dyn Iterator<Item = VertexIndex> + 'a>;

    fn neighbors(self, vertex: Self::NodeId) -> Self::Neighbors {
        Box::new(
            self.triangulation
                .vertex(self.handle(vertex))
                .out_edges()
                .map(|handle| self.vertex(handle.to().fix())),
        )
    }
}

impl<'a> visit::IntoEdges for &'a Mesh {
    type Edges = Box<dyn Iterator<Item = MeshEdgeReference> + 'a>;

    fn edges(self, a: Self::NodeId) -> Self::Edges {
        Box::new(
            self.triangulation
                .vertex(self.handle(a))
                .out_edges()
                .map(|edge| MeshEdgeReference {
                    from: self.vertex(edge.from().fix()),
                    to: self.vertex(edge.to().fix()),
                }),
        )
    }
}
