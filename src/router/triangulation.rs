use std::marker::PhantomData;

use geo::{point, Point};
use petgraph::visit::{self, NodeIndexable};
use spade::{
    handles::FixedVertexHandle, iterators::VertexIterator, DelaunayTriangulation, HasPosition,
    InsertionError,
};

use crate::{
    drawing::{rules::RulesTrait, Drawing},
    graph::GetNodeIndex,
};

pub trait GetVertexIndex<I> {
    fn vertex(&self) -> I;
}

#[derive(Debug, Clone)]
pub struct Triangulation<I: Copy + PartialEq + GetNodeIndex, W: GetVertexIndex<I> + HasPosition> {
    triangulation: DelaunayTriangulation<W>,
    vertex_to_handle: Vec<Option<FixedVertexHandle>>,
    index_marker: PhantomData<I>,
}

impl<I: Copy + PartialEq + GetNodeIndex, W: GetVertexIndex<I> + HasPosition<Scalar = f64>>
    Triangulation<I, W>
{
    pub fn new(drawing: &Drawing<impl Copy, impl RulesTrait>) -> Self {
        let mut this = Self {
            triangulation: <DelaunayTriangulation<W> as spade::Triangulation>::new(),
            vertex_to_handle: Vec::new(),
            index_marker: PhantomData,
        };
        this.vertex_to_handle
            .resize(drawing.geometry().graph().node_bound(), None);
        this
    }

    pub fn add_vertex(&mut self, weight: W) -> Result<(), InsertionError> {
        let index = weight.vertex().node_index().index();
        self.vertex_to_handle[index] = Some(spade::Triangulation::insert(
            &mut self.triangulation,
            weight,
        )?);
        Ok(())
    }

    pub fn weight(&self, vertex: I) -> &W {
        spade::Triangulation::s(&self.triangulation)
            .vertex_data(self.vertex_to_handle[vertex.node_index().index()].unwrap())
    }

    pub fn weight_mut(&mut self, vertex: I) -> &mut W {
        spade::Triangulation::vertex_data_mut(
            &mut self.triangulation,
            self.vertex_to_handle[vertex.node_index().index()].unwrap(),
        )
    }

    pub fn position(&self, vertex: I) -> Point {
        let position =
            spade::Triangulation::vertex(&self.triangulation, self.handle(vertex)).position();
        point! {x: position.x, y: position.y}
    }

    fn vertex(&self, handle: FixedVertexHandle) -> I {
        spade::Triangulation::vertex(&self.triangulation, handle)
            .as_ref()
            .vertex()
    }

    fn handle(&self, vertex: I) -> FixedVertexHandle {
        self.vertex_to_handle[vertex.node_index().index()].unwrap()
    }
}

impl<I: Copy + PartialEq + GetNodeIndex, W: GetVertexIndex<I> + HasPosition<Scalar = f64>>
    visit::GraphBase for Triangulation<I, W>
{
    type NodeId = I;
    type EdgeId = (I, I);
}

impl<I: Copy + PartialEq + GetNodeIndex, W: GetVertexIndex<I> + HasPosition<Scalar = f64>>
    visit::Data for Triangulation<I, W>
{
    type NodeWeight = ();
    type EdgeWeight = ();
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TriangulationEdgeReference<I> {
    from: I,
    to: I,
}

impl<I: Copy> visit::EdgeRef for TriangulationEdgeReference<I> {
    type NodeId = I;
    type EdgeId = (I, I);
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

impl<'a, I: Copy + PartialEq + GetNodeIndex, W: GetVertexIndex<I> + HasPosition<Scalar = f64>>
    visit::IntoNeighbors for &'a Triangulation<I, W>
{
    type Neighbors = Box<dyn Iterator<Item = I> + 'a>;

    fn neighbors(self, vertex: Self::NodeId) -> Self::Neighbors {
        Box::new(
            spade::Triangulation::vertex(&self.triangulation, self.handle(vertex))
                .out_edges()
                .map(|handle| self.vertex(handle.to().fix())),
        )
    }
}

impl<'a, I: Copy + PartialEq + GetNodeIndex, W: GetVertexIndex<I> + HasPosition<Scalar = f64>>
    visit::IntoEdgeReferences for &'a Triangulation<I, W>
{
    type EdgeRef = TriangulationEdgeReference<I>;
    type EdgeReferences = Box<dyn Iterator<Item = TriangulationEdgeReference<I>> + 'a>;

    fn edge_references(self) -> Self::EdgeReferences {
        Box::new(
            spade::Triangulation::directed_edges(&self.triangulation).map(|edge| {
                TriangulationEdgeReference {
                    from: self.vertex(edge.from().fix()),
                    to: self.vertex(edge.to().fix()),
                }
            }),
        )
    }
}

impl<'a, I: Copy + PartialEq + GetNodeIndex, W: GetVertexIndex<I> + HasPosition<Scalar = f64>>
    visit::IntoEdges for &'a Triangulation<I, W>
{
    type Edges = Box<dyn Iterator<Item = TriangulationEdgeReference<I>> + 'a>;

    fn edges(self, node: Self::NodeId) -> Self::Edges {
        Box::new(
            spade::Triangulation::vertex(&self.triangulation, self.handle(node))
                .out_edges()
                .map(|edge| TriangulationEdgeReference {
                    from: self.vertex(edge.from().fix()),
                    to: self.vertex(edge.to().fix()),
                }),
        )
    }
}

impl<'a, I: Copy + PartialEq + GetNodeIndex, W: GetVertexIndex<I> + HasPosition<Scalar = f64>>
    visit::IntoNodeIdentifiers for &'a Triangulation<I, W>
{
    type NodeIdentifiers = Box<dyn Iterator<Item = I> + 'a>;

    fn node_identifiers(self) -> Self::NodeIdentifiers {
        Box::new(
            spade::Triangulation::fixed_vertices(&self.triangulation).map(|vertex| {
                spade::Triangulation::s(&self.triangulation)
                    .vertex_data(vertex)
                    .vertex()
            }),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TriangulationVertexReference<I> {
    index: I,
}

impl<I: Copy> visit::NodeRef for TriangulationVertexReference<I> {
    type NodeId = I;
    type Weight = ();

    fn id(&self) -> Self::NodeId {
        self.index
    }

    fn weight(&self) -> &Self::Weight {
        &()
    }
}

impl<'a, I: Copy + PartialEq + GetNodeIndex, W: GetVertexIndex<I> + HasPosition<Scalar = f64>>
    visit::IntoNodeReferences for &'a Triangulation<I, W>
{
    type NodeRef = TriangulationVertexReference<I>;
    type NodeReferences = Box<dyn Iterator<Item = TriangulationVertexReference<I>> + 'a>;
    fn node_references(self) -> Self::NodeReferences {
        Box::new(
            spade::Triangulation::fixed_vertices(&self.triangulation).map(|vertex| {
                TriangulationVertexReference {
                    index: spade::Triangulation::s(&self.triangulation)
                        .vertex_data(vertex)
                        .vertex(),
                }
            }),
        )
    }
}

impl<'a, I: Copy + PartialEq + GetNodeIndex, W: GetVertexIndex<I> + HasPosition<Scalar = f64>>
    visit::NodeIndexable for &'a Triangulation<I, W>
{
    fn node_bound(&self) -> usize {
        spade::Triangulation::num_vertices(&self.triangulation)
    }

    fn to_index(&self, node: I) -> usize {
        node.node_index().index()
    }

    fn from_index(&self, index: usize) -> I {
        spade::Triangulation::s(&self.triangulation)
            .vertex_data(self.vertex_to_handle[index].unwrap())
            .vertex()
    }
}
