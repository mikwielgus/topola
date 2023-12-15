use std::iter;

use enum_dispatch::enum_dispatch;
use geo::Point;
use itertools::Itertools;
use petgraph::visit;
use petgraph::{
    stable_graph::{NodeIndex, StableDiGraph},
    visit::EdgeRef,
};
use spade::{HasPosition, InsertionError, Point2};

use crate::primitive::{GetCore, Primitive};
use crate::triangulation::TriangulationEdgeReference;
use crate::{
    graph::{
        FixedBendIndex, FixedDotIndex, GetNodeIndex, Index, Label, LooseBendIndex, MakePrimitive,
        Weight,
    },
    layout::Layout,
    primitive::MakeShape,
    shape::ShapeTrait,
    triangulation::{GetVertexIndex, Triangulation},
};

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum VertexIndex {
    FixedDot(FixedDotIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

#[derive(Debug, Clone)]
struct TriangulationWeight {
    vertex: VertexIndex,
    rails: Vec<LooseBendIndex>,
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
            let center = node.primitive(layout).shape().center();

            match node {
                Index::FixedDot(fixed_dot) => {
                    self.triangulation.add_vertex(TriangulationWeight {
                        vertex: fixed_dot.into(),
                        rails: vec![],
                        pos: center,
                    })?;
                }
                Index::FixedBend(fixed_bend) => {
                    self.triangulation.add_vertex(TriangulationWeight {
                        vertex: fixed_bend.into(),
                        rails: vec![],
                        pos: center,
                    })?;
                }
                _ => (),
            }
        }

        for node in layout.nodes() {
            match node {
                Index::LooseBend(loose_bend) => {
                    self.triangulation
                        .weight_mut(layout.primitive(loose_bend).core().into())
                        .rails
                        .push(loose_bend.into());
                }
                _ => (),
            }
        }

        Ok(())
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

impl<'a> visit::IntoNeighbors for &'a Mesh {
    type Neighbors = Box<dyn Iterator<Item = VertexIndex> + 'a>;

    fn neighbors(self, vertex: Self::NodeId) -> Self::Neighbors {
        Box::new(self.triangulation.neighbors(vertex).flat_map(|neighbor| {
            iter::once(neighbor).chain(
                self.triangulation
                    .weight(neighbor)
                    .rails
                    .iter()
                    .map(|index| VertexIndex::from(*index)),
            )
        }))
    }
}

fn edges(
    triangulation: &Triangulation<VertexIndex, TriangulationWeight>,
    edge: TriangulationEdgeReference<VertexIndex>,
) -> impl Iterator<Item = MeshEdgeReference> {
    let mut from_vertices = vec![edge.source()];
    from_vertices.extend(
        triangulation
            .weight(edge.source())
            .rails
            .iter()
            .map(|bend| VertexIndex::from(*bend)),
    );

    let mut to_vertices = vec![edge.target()];
    to_vertices.extend(
        triangulation
            .weight(edge.target())
            .rails
            .iter()
            .map(|bend| VertexIndex::from(*bend)),
    );

    from_vertices
        .into_iter()
        .cartesian_product(to_vertices.into_iter())
        .map(|pair| MeshEdgeReference {
            from: pair.0,
            to: pair.1,
        })
}

impl<'a> visit::IntoEdgeReferences for &'a Mesh {
    type EdgeRef = MeshEdgeReference;
    type EdgeReferences = Box<dyn Iterator<Item = MeshEdgeReference> + 'a>;

    fn edge_references(self) -> Self::EdgeReferences {
        Box::new(
            self.triangulation
                .edge_references()
                .flat_map(move |edge| edges(&self.triangulation, edge)),
        )
    }
}

impl<'a> visit::IntoEdges for &'a Mesh {
    type Edges = Box<dyn Iterator<Item = MeshEdgeReference> + 'a>;

    fn edges(self, node: Self::NodeId) -> Self::Edges {
        Box::new(
            self.triangulation
                .edges(node)
                .flat_map(move |edge| edges(&self.triangulation, edge)),
        )
    }
}
