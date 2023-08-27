use fixedbitset::FixedBitSet;
use geo::{point, Point};
use petgraph::{
    stable_graph::NodeIndex,
    visit::{self, NodeIndexable},
};
use spade::{
    handles::{DirectedEdgeHandle, FixedDirectedEdgeHandle, FixedVertexHandle},
    iterators::DirectedEdgeIterator,
    DelaunayTriangulation, HasPosition, InsertionError, Point2, Triangulation,
};

use crate::{graph::DotIndex, layout::Layout, router::Router};

struct Vertex {
    pub dot: DotIndex,
    x: f64,
    y: f64,
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

pub struct Mesh {
    triangulation: DelaunayTriangulation<Vertex>,
    dot_to_vertex: Vec<Option<VertexIndex>>,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            triangulation: DelaunayTriangulation::new(),
            dot_to_vertex: Vec::new(),
        }
    }

    pub fn triangulate(&mut self, layout: &Layout) -> Result<(), InsertionError> {
        self.triangulation.clear();
        self.dot_to_vertex = Vec::new();
        self.dot_to_vertex.resize(layout.graph.node_bound(), None);

        for dot in layout.dots() {
            let center = layout.primitive(dot).shape().center();

            self.dot_to_vertex[dot.index.index()] = Some(VertexIndex {
                handle: self.triangulation.insert(Vertex {
                    dot,
                    x: center.x(),
                    y: center.y(),
                })?,
            });
        }

        Ok(())
    }

    pub fn dot(&self, vertex: VertexIndex) -> DotIndex {
        self.triangulation.vertex(vertex.handle).as_ref().dot
    }

    pub fn vertex(&self, dot: DotIndex) -> VertexIndex {
        self.dot_to_vertex[dot.index.index()].unwrap()
    }

    pub fn position(&self, vertex: VertexIndex) -> Point {
        let position = self.triangulation.vertex(vertex.handle).position();
        point! {x: position.x, y: position.y}
    }
}

impl visit::GraphBase for Mesh {
    type NodeId = VertexIndex;
    type EdgeId = FixedDirectedEdgeHandle;
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
pub struct MeshEdgeReference<'a> {
    handle: DirectedEdgeHandle<'a, Vertex, (), (), ()>,
}

impl<'a> visit::EdgeRef for MeshEdgeReference<'a> {
    type NodeId = VertexIndex;
    type EdgeId = FixedDirectedEdgeHandle;
    type Weight = ();

    fn source(&self) -> Self::NodeId {
        VertexIndex {
            handle: self.handle.from().fix(),
        }
    }

    fn target(&self) -> Self::NodeId {
        VertexIndex {
            handle: self.handle.to().fix(),
        }
    }

    fn weight(&self) -> &Self::Weight {
        &()
    }

    fn id(&self) -> Self::EdgeId {
        self.handle.fix()
    }
}

pub struct MeshEdgeReferences<'a> {
    iter: DirectedEdgeIterator<'a, Vertex, (), (), ()>,
}

impl<'a> Iterator for MeshEdgeReferences<'a> {
    type Item = MeshEdgeReference<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let handle = self.iter.next()?;
        Some(MeshEdgeReference { handle })
    }
}

impl<'a> visit::IntoEdgeReferences for &'a Mesh {
    type EdgeRef = MeshEdgeReference<'a>;
    type EdgeReferences = MeshEdgeReferences<'a>;

    fn edge_references(self) -> Self::EdgeReferences {
        MeshEdgeReferences {
            iter: self.triangulation.directed_edges(),
        }
    }
}

pub struct MeshNeighbors<'a> {
    iter: Box<dyn Iterator<Item = DirectedEdgeHandle<'a, Vertex, (), (), ()>> + 'a>,
}

impl<'a> Iterator for MeshNeighbors<'a> {
    type Item = VertexIndex;

    fn next(&mut self) -> Option<Self::Item> {
        let handle = self.iter.next()?;
        Some(VertexIndex {
            handle: handle.to().fix(),
        })
    }
}

impl<'a> visit::IntoNeighbors for &'a Mesh {
    type Neighbors = MeshNeighbors<'a>;

    fn neighbors(self, a: Self::NodeId) -> Self::Neighbors {
        MeshNeighbors {
            iter: Box::new(self.triangulation.vertex(a.handle).out_edges()),
        }
    }
}

pub struct MeshEdges<'a> {
    iter: Box<dyn Iterator<Item = DirectedEdgeHandle<'a, Vertex, (), (), ()>> + 'a>,
}

impl<'a> Iterator for MeshEdges<'a> {
    type Item = MeshEdgeReference<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let handle = self.iter.next()?;
        Some(MeshEdgeReference { handle })
    }
}

impl<'a> visit::IntoEdges for &'a Mesh {
    type Edges = MeshEdges<'a>;

    fn edges(self, a: Self::NodeId) -> Self::Edges {
        MeshEdges {
            iter: Box::new(self.triangulation.vertex(a.handle).out_edges()),
        }
    }
}
