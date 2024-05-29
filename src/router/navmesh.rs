use std::iter;

use enum_dispatch::enum_dispatch;
use geo::Point;
use itertools::Itertools;
use petgraph::visit::{self, NodeIndexable};
use petgraph::{stable_graph::NodeIndex, visit::EdgeRef};
use spade::{HasPosition, InsertionError, Point2};
use thiserror::Error;

use crate::drawing::graph::{GetLayer, GetMaybeNet};
use crate::geometry::shape::ShapeTrait;
use crate::{
    drawing::{
        bend::{FixedBendIndex, LooseBendIndex},
        dot::FixedDotIndex,
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::{GetCore, MakePrimitiveShape, Primitive},
        rules::RulesTrait,
        Drawing,
    },
    graph::GetNodeIndex,
    layout::Layout,
    triangulation::{GetVertexIndex, Triangulation, TriangulationEdgeReference},
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
enum TriangulationVertexIndex {
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
struct TriangulationVertexWeight {
    vertex: TriangulationVertexIndex,
    rails: Vec<LooseBendIndex>,
    pos: Point,
}

impl GetVertexIndex<TriangulationVertexIndex> for TriangulationVertexWeight {
    fn vertex_index(&self) -> TriangulationVertexIndex {
        self.vertex
    }
}

impl HasPosition for TriangulationVertexWeight {
    type Scalar = f64;
    fn position(&self) -> Point2<Self::Scalar> {
        Point2::new(self.pos.x(), self.pos.y())
    }
}

#[derive(Debug, Clone)]
pub struct Navmesh {
    triangulation: Triangulation<TriangulationVertexIndex, TriangulationVertexWeight, ()>,
    vertex_to_triangulation_vertex: Vec<Option<TriangulationVertexIndex>>,
    from: FixedDotIndex,
    to: FixedDotIndex,
}

#[derive(Error, Debug, Clone)]
pub enum NavmeshError {
    #[error("failed to insert vertex in navmesh")]
    Insertion(#[from] InsertionError),
}

impl Navmesh {
    pub fn new(
        layout: &Layout<impl RulesTrait>,
        from: FixedDotIndex,
        to: FixedDotIndex,
    ) -> Result<Self, NavmeshError> {
        let mut this = Self {
            triangulation: Triangulation::new(layout.drawing().geometry().graph().node_bound()),
            vertex_to_triangulation_vertex: Vec::new(),
            from,
            to,
        };
        this.vertex_to_triangulation_vertex
            .resize(layout.drawing().geometry().graph().node_bound(), None);

        let layer = layout.drawing().primitive(from).layer();
        let maybe_net = layout.drawing().primitive(from).maybe_net();

        for node in layout.drawing().layer_primitive_nodes(layer) {
            let primitive = node.primitive(layout.drawing());

            if let Some(primitive_net) = primitive.maybe_net() {
                if node == from.into() || node == to.into() || Some(primitive_net) != maybe_net {
                    match node {
                        PrimitiveIndex::FixedDot(dot) => {
                            this.triangulation.add_vertex(TriangulationVertexWeight {
                                vertex: dot.into(),
                                rails: vec![],
                                pos: primitive.shape().center(),
                            })?;
                        }
                        PrimitiveIndex::FixedBend(bend) => {
                            this.triangulation.add_vertex(TriangulationVertexWeight {
                                vertex: bend.into(),
                                rails: vec![],
                                pos: primitive.shape().center(),
                            })?;
                        }
                        _ => (),
                    }
                }
            }
        }

        for node in layout.drawing().layer_primitive_nodes(layer) {
            // Add rails as vertices. This is how the navmesh differs from the triangulation.
            match node {
                PrimitiveIndex::LooseBend(bend) => {
                    this.triangulation
                        .weight_mut(layout.drawing().primitive(bend).core().into())
                        .rails
                        .push(bend.into());
                    this.vertex_to_triangulation_vertex[bend.node_index().index()] =
                        Some(layout.drawing().primitive(bend).core().into());
                }
                _ => (),
            }
        }

        Ok(this)
    }

    fn triangulation_vertex(&self, vertex: VertexIndex) -> TriangulationVertexIndex {
        match vertex {
            VertexIndex::FixedDot(dot) => TriangulationVertexIndex::FixedDot(dot),
            VertexIndex::FixedBend(bend) => TriangulationVertexIndex::FixedBend(bend),
            VertexIndex::LooseBend(bend) => {
                self.vertex_to_triangulation_vertex[bend.node_index().index()].unwrap()
            }
        }
    }

    pub fn from(&self) -> FixedDotIndex {
        self.from
    }

    pub fn to(&self) -> FixedDotIndex {
        self.to
    }
}

impl visit::GraphBase for Navmesh {
    type NodeId = VertexIndex;
    type EdgeId = (VertexIndex, VertexIndex);
}

impl visit::Data for Navmesh {
    type NodeWeight = ();
    type EdgeWeight = ();
}

#[derive(Debug, Clone, Copy)]
pub struct NavmeshEdgeReference {
    from: VertexIndex,
    to: VertexIndex,
}

impl visit::EdgeRef for NavmeshEdgeReference {
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

impl<'a> visit::IntoNeighbors for &'a Navmesh {
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
    triangulation: &Triangulation<TriangulationVertexIndex, TriangulationVertexWeight, ()>,
    edge: TriangulationEdgeReference<TriangulationVertexIndex, ()>,
) -> impl Iterator<Item = NavmeshEdgeReference> {
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
        .map(|pair| NavmeshEdgeReference {
            from: pair.0,
            to: pair.1.into(),
        })
}

impl<'a> visit::IntoEdgeReferences for &'a Navmesh {
    type EdgeRef = NavmeshEdgeReference;
    type EdgeReferences = Box<dyn Iterator<Item = NavmeshEdgeReference> + 'a>;

    fn edge_references(self) -> Self::EdgeReferences {
        Box::new(
            self.triangulation
                .edge_references()
                .flat_map(move |edge| edge_with_near_edges(&self.triangulation, edge)),
        )
    }
}

fn vertex_edges(
    triangulation: &Triangulation<TriangulationVertexIndex, TriangulationVertexWeight, ()>,
    from: VertexIndex,
    to: TriangulationVertexIndex,
) -> impl Iterator<Item = NavmeshEdgeReference> {
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
        .map(|pair| NavmeshEdgeReference {
            from: pair.0,
            to: pair.1,
        })
}

impl<'a> visit::IntoEdges for &'a Navmesh {
    type Edges = Box<dyn Iterator<Item = NavmeshEdgeReference> + 'a>;

    fn edges(self, vertex: Self::NodeId) -> Self::Edges {
        Box::new(
            self.triangulation
                .edges(self.triangulation_vertex(vertex))
                .flat_map(move |edge| vertex_edges(&self.triangulation, vertex, edge.target())),
        )
    }
}
