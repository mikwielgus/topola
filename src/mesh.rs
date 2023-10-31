use enum_dispatch::enum_dispatch;
use fixedbitset::FixedBitSet;
use geo::{point, Point};
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::{self, NodeIndexable};
use spade::{
    handles::{DirectedEdgeHandle, FixedDirectedEdgeHandle, FixedVertexHandle},
    iterators::DirectedEdgeIterator,
    DelaunayTriangulation, HasPosition, InsertionError, Point2, Triangulation,
};

use crate::{
    graph::{FixedBendIndex, FixedDotIndex, GetNodeIndex, Index, LooseBendIndex},
    layout::Layout,
};
use crate::{primitive::MakeShape, shape::ShapeTrait};

#[derive(Debug, Clone)]
struct Vertex {
    graph_index: VertexGraphIndex,
    x: f64,
    y: f64,
}

#[enum_dispatch(GetNodeIndex)]
#[derive(Debug, Clone, Copy)]
pub enum VertexGraphIndex {
    FixedDot(FixedDotIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub struct VertexIndex {
    handle: FixedVertexHandle,
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
    graph_index_to_vertex: Vec<Option<VertexIndex>>,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            triangulation: DelaunayTriangulation::new(),
            graph_index_to_vertex: Vec::new(),
        }
    }

    pub fn triangulate(&mut self, layout: &Layout) -> Result<(), InsertionError> {
        self.triangulation.clear();
        self.graph_index_to_vertex = Vec::new();
        self.graph_index_to_vertex
            .resize(layout.graph.node_bound(), None);

        for node in layout.nodes() {
            if let Index::FixedDot(dot) = node {
                let center = layout.primitive(dot).shape().center();

                self.graph_index_to_vertex[dot.node_index().index()] = Some(VertexIndex {
                    handle: self.triangulation.insert(Vertex {
                        graph_index: dot.into(),
                        x: center.x(),
                        y: center.y(),
                    })?,
                });
            }
        }

        Ok(())
    }

    pub fn graph_index(&self, vertex: VertexIndex) -> VertexGraphIndex {
        self.triangulation
            .vertex(vertex.handle)
            .as_ref()
            .graph_index
    }

    pub fn vertex(&self, graph_index: VertexGraphIndex) -> VertexIndex {
        self.graph_index_to_vertex[graph_index.node_index().index()].unwrap()
    }

    pub fn position(&self, vertex: VertexIndex) -> Point {
        let position = self.triangulation.vertex(vertex.handle).position();
        point! {x: position.x, y: position.y}
    }
}

impl visit::GraphBase for Mesh {
    type NodeId = VertexIndex;
    type EdgeId = (VertexIndex, VertexIndex);
}

pub struct MeshVisitMap {
    fixedbitset: FixedBitSet,
}

impl MeshVisitMap {
    pub fn with_capacity(bits: usize) -> Self {
        Self {
            fixedbitset: FixedBitSet::with_capacity(bits),
        }
    }

    pub fn clear(&mut self) {
        self.fixedbitset.clear();
    }

    pub fn grow(&mut self, bits: usize) {
        self.fixedbitset.grow(bits);
    }
}

pub trait IndexHolder {
    fn index(&self) -> usize;
}

impl IndexHolder for VertexIndex {
    fn index(&self) -> usize {
        self.handle.index()
    }
}

impl<T: IndexHolder> visit::VisitMap<T> for MeshVisitMap {
    fn visit(&mut self, a: T) -> bool {
        !self.fixedbitset.put(a.index())
    }

    fn is_visited(&self, a: &T) -> bool {
        self.fixedbitset.contains(a.index())
    }
}

impl visit::Visitable for Mesh {
    type Map = MeshVisitMap;

    fn visit_map(&self) -> Self::Map {
        MeshVisitMap::with_capacity(self.triangulation.num_vertices())
    }

    fn reset_map(&self, map: &mut Self::Map) {
        map.clear();
        map.grow(self.triangulation.num_vertices());
    }
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
                    from: VertexIndex {
                        handle: edge.from().fix(),
                    },
                    to: VertexIndex {
                        handle: edge.to().fix(),
                    },
                }),
        )
    }
}

impl<'a> visit::IntoNeighbors for &'a Mesh {
    type Neighbors = Box<dyn Iterator<Item = VertexIndex> + 'a>;

    fn neighbors(self, a: Self::NodeId) -> Self::Neighbors {
        Box::new(
            self.triangulation
                .vertex(a.handle)
                .out_edges()
                .map(|handle| VertexIndex {
                    handle: handle.to().fix(),
                }),
        )
    }
}

impl<'a> visit::IntoEdges for &'a Mesh {
    type Edges = Box<dyn Iterator<Item = MeshEdgeReference> + 'a>;

    fn edges(self, a: Self::NodeId) -> Self::Edges {
        Box::new(
            self.triangulation
                .vertex(a.handle)
                .out_edges()
                .map(|edge| MeshEdgeReference {
                    from: VertexIndex {
                        handle: edge.from().fix(),
                    },
                    to: VertexIndex {
                        handle: edge.to().fix(),
                    },
                }),
        )
    }
}
