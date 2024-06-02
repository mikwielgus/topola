use std::{cmp::Ordering, marker::PhantomData};

use geo::{point, EuclideanDistance, Point};
use petgraph::visit;
use spade::{handles::FixedVertexHandle, DelaunayTriangulation, HasPosition, InsertionError};

use crate::graph::GetNodeIndex;

pub trait GetTrianvertexIndex<I> {
    fn trianvertex_index(&self) -> I;
}

#[derive(Debug, Clone)]
pub struct Triangulation<
    I: Copy + PartialEq + GetNodeIndex,
    VW: GetTrianvertexIndex<I> + HasPosition,
    EW: Copy + Default,
> {
    triangulation: DelaunayTriangulation<VW, EW>,
    trianvertex_to_handle: Vec<Option<FixedVertexHandle>>,
    index_marker: PhantomData<I>,
}

impl<
        I: Copy + PartialEq + GetNodeIndex,
        VW: GetTrianvertexIndex<I> + HasPosition<Scalar = f64>,
        EW: Copy + Default,
    > Triangulation<I, VW, EW>
{
    pub fn new(node_bound: usize) -> Self {
        let mut this = Self {
            triangulation: <DelaunayTriangulation<VW, EW> as spade::Triangulation>::new(),
            trianvertex_to_handle: Vec::new(),
            index_marker: PhantomData,
        };
        this.trianvertex_to_handle.resize(node_bound, None);
        this
    }

    pub fn add_vertex(&mut self, weight: VW) -> Result<(), InsertionError> {
        let index = weight.trianvertex_index().node_index().index();
        self.trianvertex_to_handle[index] = Some(spade::Triangulation::insert(
            &mut self.triangulation,
            weight,
        )?);
        Ok(())
    }

    pub fn weight(&self, vertex: I) -> &VW {
        spade::Triangulation::s(&self.triangulation)
            .vertex_data(self.trianvertex_to_handle[vertex.node_index().index()].unwrap())
    }

    pub fn weight_mut(&mut self, vertex: I) -> &mut VW {
        spade::Triangulation::vertex_data_mut(
            &mut self.triangulation,
            self.trianvertex_to_handle[vertex.node_index().index()].unwrap(),
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
            .trianvertex_index()
    }

    fn handle(&self, vertex: I) -> FixedVertexHandle {
        self.trianvertex_to_handle[vertex.node_index().index()].unwrap()
    }
}

impl<
        I: Copy + PartialEq + GetNodeIndex,
        VW: GetTrianvertexIndex<I> + HasPosition<Scalar = f64>,
        EW: Copy + Default,
    > visit::GraphBase for Triangulation<I, VW, EW>
{
    type NodeId = I;
    type EdgeId = (I, I);
}

#[derive(Debug, Clone, Copy)]
pub struct TriangulationEdgeWeightWrapper<EW: Copy + Default> {
    length: f64,
    pub weight: EW,
}

impl<EW: Copy + Default> PartialEq for TriangulationEdgeWeightWrapper<EW> {
    fn eq(&self, other: &Self) -> bool {
        self.length.eq(&other.length)
    }
}

impl<EW: Copy + Default> PartialOrd for TriangulationEdgeWeightWrapper<EW> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.length.partial_cmp(&other.length)
    }
}

impl<
        I: Copy + PartialEq + GetNodeIndex,
        VW: GetTrianvertexIndex<I> + HasPosition<Scalar = f64>,
        EW: Copy + Default,
    > visit::Data for Triangulation<I, VW, EW>
{
    type NodeWeight = VW;
    type EdgeWeight = TriangulationEdgeWeightWrapper<EW>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TriangulationEdgeReference<I, EW: Copy + Default> {
    from: I,
    to: I,
    weight: TriangulationEdgeWeightWrapper<EW>,
}

impl<I: Copy, EW: Copy + Default> visit::EdgeRef for TriangulationEdgeReference<I, EW> {
    type NodeId = I;
    type EdgeId = (I, I);
    type Weight = TriangulationEdgeWeightWrapper<EW>;

    fn source(&self) -> Self::NodeId {
        self.from
    }

    fn target(&self) -> Self::NodeId {
        self.to
    }

    fn weight(&self) -> &Self::Weight {
        &self.weight
    }

    fn id(&self) -> Self::EdgeId {
        (self.from, self.to)
    }
}

impl<
        'a,
        I: Copy + PartialEq + GetNodeIndex,
        VW: GetTrianvertexIndex<I> + HasPosition<Scalar = f64>,
        EW: Copy + Default,
    > visit::IntoNeighbors for &'a Triangulation<I, VW, EW>
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

impl<
        'a,
        I: Copy + PartialEq + GetNodeIndex,
        VW: GetTrianvertexIndex<I> + HasPosition<Scalar = f64>,
        EW: Copy + Default,
    > visit::IntoEdgeReferences for &'a Triangulation<I, VW, EW>
{
    type EdgeRef = TriangulationEdgeReference<I, EW>;
    type EdgeReferences = Box<dyn Iterator<Item = TriangulationEdgeReference<I, EW>> + 'a>;

    fn edge_references(self) -> Self::EdgeReferences {
        Box::new(
            spade::Triangulation::directed_edges(&self.triangulation).map(|edge| {
                let from = self.vertex(edge.from().fix());
                let to = self.vertex(edge.to().fix());

                TriangulationEdgeReference {
                    from,
                    to,
                    weight: TriangulationEdgeWeightWrapper {
                        length: self.position(from).euclidean_distance(&self.position(to)),
                        weight: *edge.data(),
                    },
                }
            }),
        )
    }
}

impl<
        'a,
        I: Copy + PartialEq + GetNodeIndex,
        VW: GetTrianvertexIndex<I> + HasPosition<Scalar = f64>,
        EW: Copy + Default,
    > visit::IntoEdges for &'a Triangulation<I, VW, EW>
{
    type Edges = Box<dyn Iterator<Item = TriangulationEdgeReference<I, EW>> + 'a>;

    fn edges(self, node: Self::NodeId) -> Self::Edges {
        Box::new(
            spade::Triangulation::vertex(&self.triangulation, self.handle(node))
                .out_edges()
                .map(|edge| {
                    let from = self.vertex(edge.from().fix());
                    let to = self.vertex(edge.to().fix());

                    TriangulationEdgeReference {
                        from,
                        to,
                        weight: TriangulationEdgeWeightWrapper {
                            length: self.position(from).euclidean_distance(&self.position(to)),
                            weight: *edge.data(),
                        },
                    }
                }),
        )
    }
}

impl<
        'a,
        I: Copy + PartialEq + GetNodeIndex,
        VW: GetTrianvertexIndex<I> + HasPosition<Scalar = f64>,
        EW: Copy + Default,
    > visit::IntoNodeIdentifiers for &'a Triangulation<I, VW, EW>
{
    type NodeIdentifiers = Box<dyn Iterator<Item = I> + 'a>;

    fn node_identifiers(self) -> Self::NodeIdentifiers {
        Box::new(
            spade::Triangulation::fixed_vertices(&self.triangulation).map(|vertex| {
                spade::Triangulation::s(&self.triangulation)
                    .vertex_data(vertex)
                    .trianvertex_index()
            }),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TriangulationVertexReference<'a, I: Copy, VW> {
    index: I,
    weight: &'a VW,
}

impl<'a, I: Copy, VW: Copy> visit::NodeRef for TriangulationVertexReference<'a, I, VW> {
    type NodeId = I;
    type Weight = VW;

    fn id(&self) -> Self::NodeId {
        self.index
    }

    fn weight(&self) -> &Self::Weight {
        self.weight
    }
}

impl<
        'a,
        I: Copy + PartialEq + GetNodeIndex,
        VW: Copy + GetTrianvertexIndex<I> + HasPosition<Scalar = f64>,
        EW: Copy + Default,
    > visit::IntoNodeReferences for &'a Triangulation<I, VW, EW>
{
    type NodeRef = TriangulationVertexReference<'a, I, VW>;
    type NodeReferences = Box<dyn Iterator<Item = TriangulationVertexReference<'a, I, VW>> + 'a>;

    fn node_references(self) -> Self::NodeReferences {
        Box::new(
            spade::Triangulation::fixed_vertices(&self.triangulation).map(|vertex| {
                let weight = spade::Triangulation::s(&self.triangulation).vertex_data(vertex);
                TriangulationVertexReference {
                    index: weight.trianvertex_index(),
                    weight,
                }
            }),
        )
    }
}

impl<
        'a,
        I: Copy + PartialEq + GetNodeIndex + std::fmt::Debug,
        VW: GetTrianvertexIndex<I> + HasPosition<Scalar = f64>,
        EW: Copy + Default,
    > visit::NodeIndexable for &'a Triangulation<I, VW, EW>
{
    fn node_bound(&self) -> usize {
        //spade::Triangulation::num_vertices(&self.triangulation)
        self.trianvertex_to_handle.len()
    }

    fn to_index(&self, node: I) -> usize {
        node.node_index().index()
    }

    fn from_index(&self, index: usize) -> I {
        spade::Triangulation::s(&self.triangulation)
            .vertex_data(self.trianvertex_to_handle[index].unwrap())
            .trianvertex_index()
    }
}
