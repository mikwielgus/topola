use geo::{point, Point};
use petgraph::visit::{self, NodeIndexable};
use spade::{
    handles::FixedVertexHandle, DelaunayTriangulation, HasPosition, InsertionError, Point2,
};

use crate::{
    graph::GetNodeIndex,
    layout::Layout,
    mesh::{MeshEdgeReference, VertexIndex},
};

#[derive(Debug, Clone)]
struct VertexWeight {
    vertex: VertexIndex,
    x: f64,
    y: f64,
}

impl HasPosition for VertexWeight {
    type Scalar = f64;
    fn position(&self) -> Point2<Self::Scalar> {
        Point2::new(self.x, self.y)
    }
}

#[derive(Debug, Clone)]
pub struct Triangulation {
    triangulation: DelaunayTriangulation<VertexWeight>,
    vertex_to_handle: Vec<Option<FixedVertexHandle>>,
}

impl Triangulation {
    pub fn new(layout: &Layout) -> Self {
        let mut this = Self {
            triangulation: <DelaunayTriangulation<VertexWeight> as spade::Triangulation>::new(),
            vertex_to_handle: Vec::new(),
        };
        this.vertex_to_handle
            .resize(layout.graph.node_bound(), None);
        this
    }

    pub fn add_vertex(
        &mut self,
        vertex: VertexIndex,
        x: f64,
        y: f64,
    ) -> Result<(), InsertionError> {
        self.vertex_to_handle[vertex.node_index().index()] = Some(spade::Triangulation::insert(
            &mut self.triangulation,
            VertexWeight { vertex, x, y },
        )?);
        Ok(())
    }

    pub fn project_vertex(&mut self, from: VertexIndex, to: VertexIndex) {
        self.vertex_to_handle[from.node_index().index()] =
            self.vertex_to_handle[to.node_index().index()]
    }

    pub fn position(&self, vertex: VertexIndex) -> Point {
        let position =
            spade::Triangulation::vertex(&self.triangulation, self.handle(vertex)).position();
        point! {x: position.x, y: position.y}
    }

    fn vertex(&self, handle: FixedVertexHandle) -> VertexIndex {
        spade::Triangulation::vertex(&self.triangulation, handle)
            .as_ref()
            .vertex
    }

    fn handle(&self, vertex: VertexIndex) -> FixedVertexHandle {
        self.vertex_to_handle[vertex.node_index().index()].unwrap()
    }
}

impl visit::GraphBase for Triangulation {
    type NodeId = VertexIndex;
    type EdgeId = (VertexIndex, VertexIndex);
}

impl visit::Data for Triangulation {
    type NodeWeight = ();
    type EdgeWeight = ();
}

impl<'a> visit::IntoEdgeReferences for &'a Triangulation {
    type EdgeRef = MeshEdgeReference;
    type EdgeReferences = Box<dyn Iterator<Item = MeshEdgeReference> + 'a>;

    fn edge_references(self) -> Self::EdgeReferences {
        Box::new(
            spade::Triangulation::directed_edges(&self.triangulation).map(|edge| {
                MeshEdgeReference {
                    from: self.vertex(edge.from().fix()),
                    to: self.vertex(edge.to().fix()),
                }
            }),
        )
    }
}

impl<'a> visit::IntoNeighbors for &'a Triangulation {
    type Neighbors = Box<dyn Iterator<Item = VertexIndex> + 'a>;

    fn neighbors(self, vertex: Self::NodeId) -> Self::Neighbors {
        Box::new(
            spade::Triangulation::vertex(&self.triangulation, self.handle(vertex))
                .out_edges()
                .map(|handle| self.vertex(handle.to().fix())),
        )
    }
}

impl<'a> visit::IntoEdges for &'a Triangulation {
    type Edges = Box<dyn Iterator<Item = MeshEdgeReference> + 'a>;

    fn edges(self, node: Self::NodeId) -> Self::Edges {
        Box::new(
            spade::Triangulation::vertex(&self.triangulation, self.handle(node))
                .out_edges()
                .map(|edge| MeshEdgeReference {
                    from: self.vertex(edge.from().fix()),
                    to: self.vertex(edge.to().fix()),
                }),
        )
    }
}
