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
    triangulation::{GetTrianvertexIndex, Triangulation, TriangulationEdgeReference},
};

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum NavvertexIndex {
    FixedDot(FixedDotIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
enum TrianvertexIndex {
    FixedDot(FixedDotIndex),
    FixedBend(FixedBendIndex),
}

impl From<NavvertexIndex> for PrimitiveIndex {
    fn from(vertex: NavvertexIndex) -> Self {
        match vertex {
            NavvertexIndex::FixedDot(dot) => PrimitiveIndex::FixedDot(dot),
            NavvertexIndex::FixedBend(bend) => PrimitiveIndex::FixedBend(bend),
            NavvertexIndex::LooseBend(bend) => PrimitiveIndex::LooseBend(bend),
        }
    }
}

impl From<TrianvertexIndex> for NavvertexIndex {
    fn from(vertex: TrianvertexIndex) -> Self {
        match vertex {
            TrianvertexIndex::FixedDot(dot) => NavvertexIndex::FixedDot(dot),
            TrianvertexIndex::FixedBend(bend) => NavvertexIndex::FixedBend(bend),
        }
    }
}

#[derive(Debug, Clone)]
struct TrianvertexWeight {
    trianvertex: TrianvertexIndex,
    rails: Vec<LooseBendIndex>,
    pos: Point,
}

impl GetTrianvertexIndex<TrianvertexIndex> for TrianvertexWeight {
    fn trianvertex_index(&self) -> TrianvertexIndex {
        self.trianvertex
    }
}

impl HasPosition for TrianvertexWeight {
    type Scalar = f64;
    fn position(&self) -> Point2<Self::Scalar> {
        Point2::new(self.pos.x(), self.pos.y())
    }
}

#[derive(Debug, Clone)]
pub struct Navmesh {
    triangulation: Triangulation<TrianvertexIndex, TrianvertexWeight, ()>,
    navvertex_to_trianvertex: Vec<Option<TrianvertexIndex>>,
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
            navvertex_to_trianvertex: Vec::new(),
            from,
            to,
        };
        this.navvertex_to_trianvertex
            .resize(layout.drawing().geometry().graph().node_bound(), None);

        let layer = layout.drawing().primitive(from).layer();
        let maybe_net = layout.drawing().primitive(from).maybe_net();

        for node in layout.drawing().layer_primitive_nodes(layer) {
            let primitive = node.primitive(layout.drawing());

            if let Some(primitive_net) = primitive.maybe_net() {
                if node == from.into() || node == to.into() || Some(primitive_net) != maybe_net {
                    match node {
                        PrimitiveIndex::FixedDot(dot) => {
                            this.triangulation.add_vertex(TrianvertexWeight {
                                trianvertex: dot.into(),
                                rails: vec![],
                                pos: primitive.shape().center(),
                            })?;
                        }
                        PrimitiveIndex::FixedBend(bend) => {
                            this.triangulation.add_vertex(TrianvertexWeight {
                                trianvertex: bend.into(),
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
                    this.navvertex_to_trianvertex[bend.node_index().index()] =
                        Some(layout.drawing().primitive(bend).core().into());
                }
                _ => (),
            }
        }

        Ok(this)
    }

    fn triangulation_vertex(&self, vertex: NavvertexIndex) -> TrianvertexIndex {
        match vertex {
            NavvertexIndex::FixedDot(dot) => TrianvertexIndex::FixedDot(dot),
            NavvertexIndex::FixedBend(bend) => TrianvertexIndex::FixedBend(bend),
            NavvertexIndex::LooseBend(bend) => {
                self.navvertex_to_trianvertex[bend.node_index().index()].unwrap()
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
    type NodeId = NavvertexIndex;
    type EdgeId = (NavvertexIndex, NavvertexIndex);
}

impl visit::Data for Navmesh {
    type NodeWeight = ();
    type EdgeWeight = ();
}

#[derive(Debug, Clone, Copy)]
pub struct NavmeshEdgeReference {
    from: NavvertexIndex,
    to: NavvertexIndex,
}

impl visit::EdgeRef for NavmeshEdgeReference {
    type NodeId = NavvertexIndex;
    type EdgeId = (NavvertexIndex, NavvertexIndex);
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
    type Neighbors = Box<dyn Iterator<Item = NavvertexIndex> + 'a>;

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
                            .map(|index| NavvertexIndex::from(*index)),
                    )
                }),
        )
    }
}

fn edge_with_near_edges(
    triangulation: &Triangulation<TrianvertexIndex, TrianvertexWeight, ()>,
    edge: TriangulationEdgeReference<TrianvertexIndex, ()>,
) -> impl Iterator<Item = NavmeshEdgeReference> {
    let mut from_vertices = vec![edge.source().into()];

    // Append rails to the source.
    from_vertices.extend(
        triangulation
            .weight(edge.source())
            .rails
            .iter()
            .map(|bend| NavvertexIndex::from(*bend)),
    );

    let mut to_vertices = vec![edge.target().into()];

    // Append rails to the target.
    to_vertices.extend(
        triangulation
            .weight(edge.target())
            .rails
            .iter()
            .map(|bend| NavvertexIndex::from(*bend)),
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
    triangulation: &Triangulation<TrianvertexIndex, TrianvertexWeight, ()>,
    from: NavvertexIndex,
    to: TrianvertexIndex,
) -> impl Iterator<Item = NavmeshEdgeReference> {
    let from_vertices = vec![from];
    let mut to_vertices = vec![to.into()];

    // Append rails to the target.
    to_vertices.extend(
        triangulation
            .weight(to)
            .rails
            .iter()
            .map(|bend| NavvertexIndex::from(*bend)),
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
