use std::iter;

use enum_dispatch::enum_dispatch;
use geo::Point;
use itertools::Itertools;
use petgraph::visit::{self, NodeIndexable};
use petgraph::{stable_graph::NodeIndex, visit::EdgeRef};
use spade::{HasPosition, InsertionError, Point2};

use crate::drawing::rules::RulesTrait;
use crate::triangulation::TriangulationEdgeReference;
use crate::{
    drawing::{
        bend::{FixedBendIndex, LooseBendIndex},
        dot::FixedDotIndex,
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::{GetCore, MakeShape, Primitive},
        Drawing,
    },
    geometry::shape::ShapeTrait,
    graph::GetNodeIndex,
    triangulation::{GetVertexIndex, Triangulation},
};

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum VertexIndex {
    FixedDot(FixedDotIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum TriangulationVertexIndex {
    FixedDot(FixedDotIndex),
    FixedBend(FixedBendIndex),
}

impl From<VertexIndex> for PrimitiveIndex {
    fn from(vertex: VertexIndex) -> Self {
        match vertex {
            VertexIndex::FixedDot(dot) => PrimitiveIndex::FixedDot(dot),
            VertexIndex::FixedBend(bend) => PrimitiveIndex::FixedBend(bend),
            VertexIndex::LooseBend(bend) => PrimitiveIndex::LooseBend(bend),
        }
    }
}

impl From<TriangulationVertexIndex> for VertexIndex {
    fn from(vertex: TriangulationVertexIndex) -> Self {
        match vertex {
            TriangulationVertexIndex::FixedDot(dot) => VertexIndex::FixedDot(dot),
            TriangulationVertexIndex::FixedBend(bend) => VertexIndex::FixedBend(bend),
        }
    }
}

#[derive(Debug, Clone)]
struct TriangulationWeight {
    vertex: TriangulationVertexIndex,
    rails: Vec<LooseBendIndex>,
    pos: Point,
}

impl GetVertexIndex<TriangulationVertexIndex> for TriangulationWeight {
    fn vertex(&self) -> TriangulationVertexIndex {
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
    triangulation: Triangulation<TriangulationVertexIndex, TriangulationWeight>,
    vertex_to_triangulation_vertex: Vec<Option<TriangulationVertexIndex>>,
}

impl Mesh {
    pub fn new(layout: &Drawing<impl RulesTrait>) -> Self {
        let mut this = Self {
            triangulation: Triangulation::new(layout),
            vertex_to_triangulation_vertex: Vec::new(),
        };
        this.vertex_to_triangulation_vertex
            .resize(layout.geometry().graph().node_bound(), None);
        this
    }

    pub fn generate(&mut self, drawing: &Drawing<impl RulesTrait>) -> Result<(), InsertionError> {
        for node in drawing.primitive_nodes() {
            let center = node.primitive(drawing).shape().center();

            match node {
                PrimitiveIndex::FixedDot(dot) => {
                    self.triangulation.add_vertex(TriangulationWeight {
                        vertex: dot.into(),
                        rails: vec![],
                        pos: center,
                    })?;
                }
                PrimitiveIndex::FixedBend(bend) => {
                    self.triangulation.add_vertex(TriangulationWeight {
                        vertex: bend.into(),
                        rails: vec![],
                        pos: center,
                    })?;
                }
                _ => (),
            }
        }

        for node in drawing.primitive_nodes() {
            // Add rails as vertices. This is how the mesh differs from the triangulation.
            match node {
                PrimitiveIndex::LooseBend(bend) => {
                    self.triangulation
                        .weight_mut(drawing.primitive(bend).core().into())
                        .rails
                        .push(bend.into());
                    self.vertex_to_triangulation_vertex[bend.node_index().index()] =
                        Some(drawing.primitive(bend).core().into());
                }
                _ => (),
            }
        }

        Ok(())
    }

    pub fn triangulation_vertex(&self, vertex: VertexIndex) -> TriangulationVertexIndex {
        match vertex {
            VertexIndex::FixedDot(dot) => TriangulationVertexIndex::FixedDot(dot),
            VertexIndex::FixedBend(bend) => TriangulationVertexIndex::FixedBend(bend),
            VertexIndex::LooseBend(bend) => {
                self.vertex_to_triangulation_vertex[bend.node_index().index()].unwrap()
            }
        }
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
        Box::new(
            self.triangulation
                .neighbors(self.triangulation_vertex(vertex))
                .flat_map(|neighbor| {
                    iter::once(neighbor.into()).chain(
                        self.triangulation
                            .weight(neighbor)
                            .rails
                            .iter()
                            .map(|index| VertexIndex::from(*index)),
                    )
                }),
        )
    }
}

fn edge_with_near_edges(
    triangulation: &Triangulation<TriangulationVertexIndex, TriangulationWeight>,
    edge: TriangulationEdgeReference<TriangulationVertexIndex>,
) -> impl Iterator<Item = MeshEdgeReference> {
    let mut from_vertices = vec![edge.source().into()];

    // Append rails to the source.
    from_vertices.extend(
        triangulation
            .weight(edge.source())
            .rails
            .iter()
            .map(|bend| VertexIndex::from(*bend)),
    );

    let mut to_vertices = vec![edge.target().into()];

    // Append rails to the target.
    to_vertices.extend(
        triangulation
            .weight(edge.target())
            .rails
            .iter()
            .map(|bend| VertexIndex::from(*bend)),
    );

    // Return cartesian product.
    from_vertices
        .into_iter()
        .cartesian_product(to_vertices.into_iter())
        .map(|pair| MeshEdgeReference {
            from: pair.0,
            to: pair.1.into(),
        })
}

impl<'a> visit::IntoEdgeReferences for &'a Mesh {
    type EdgeRef = MeshEdgeReference;
    type EdgeReferences = Box<dyn Iterator<Item = MeshEdgeReference> + 'a>;

    fn edge_references(self) -> Self::EdgeReferences {
        Box::new(
            self.triangulation
                .edge_references()
                .flat_map(move |edge| edge_with_near_edges(&self.triangulation, edge)),
        )
    }
}

fn vertex_edges(
    triangulation: &Triangulation<TriangulationVertexIndex, TriangulationWeight>,
    from: VertexIndex,
    to: TriangulationVertexIndex,
) -> impl Iterator<Item = MeshEdgeReference> {
    let from_vertices = vec![from];
    let mut to_vertices = vec![to.into()];

    // Append rails to the target.
    to_vertices.extend(
        triangulation
            .weight(to)
            .rails
            .iter()
            .map(|bend| VertexIndex::from(*bend)),
    );

    // Return cartesian product.
    from_vertices
        .into_iter()
        .cartesian_product(to_vertices.into_iter())
        .map(|pair| MeshEdgeReference {
            from: pair.0,
            to: pair.1,
        })
}

impl<'a> visit::IntoEdges for &'a Mesh {
    type Edges = Box<dyn Iterator<Item = MeshEdgeReference> + 'a>;

    fn edges(self, vertex: Self::NodeId) -> Self::Edges {
        Box::new(
            self.triangulation
                .edges(self.triangulation_vertex(vertex))
                .flat_map(move |edge| vertex_edges(&self.triangulation, vertex, edge.target())),
        )
    }
}
