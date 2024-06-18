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
    graph::GetPetgraphIndex,
    layout::Layout,
    triangulation::{GetTrianvertexNodeIndex, Triangulation, TriangulationEdgeReference},
};

#[enum_dispatch(GetPetgraphIndex, MakePrimitive)]
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum NavvertexNodeIndex {
    FixedDot(FixedDotIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

#[enum_dispatch(GetPetgraphIndex, MakePrimitive)]
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
enum TrianvertexNodeIndex {
    FixedDot(FixedDotIndex),
    FixedBend(FixedBendIndex),
}

impl From<NavvertexNodeIndex> for PrimitiveIndex {
    fn from(vertex: NavvertexNodeIndex) -> Self {
        match vertex {
            NavvertexNodeIndex::FixedDot(dot) => PrimitiveIndex::FixedDot(dot),
            NavvertexNodeIndex::FixedBend(bend) => PrimitiveIndex::FixedBend(bend),
            NavvertexNodeIndex::LooseBend(bend) => PrimitiveIndex::LooseBend(bend),
        }
    }
}

impl From<TrianvertexNodeIndex> for NavvertexNodeIndex {
    fn from(vertex: TrianvertexNodeIndex) -> Self {
        match vertex {
            TrianvertexNodeIndex::FixedDot(dot) => NavvertexNodeIndex::FixedDot(dot),
            TrianvertexNodeIndex::FixedBend(bend) => NavvertexNodeIndex::FixedBend(bend),
        }
    }
}

#[derive(Debug, Clone)]
struct TrianvertexWeight {
    node: TrianvertexNodeIndex,
    rails: Vec<LooseBendIndex>,
    pos: Point,
}

impl GetTrianvertexNodeIndex<TrianvertexNodeIndex> for TrianvertexWeight {
    fn node_index(&self) -> TrianvertexNodeIndex {
        self.node
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
    triangulation: Triangulation<TrianvertexNodeIndex, TrianvertexWeight, ()>,
    navvertex_to_trianvertex: Vec<Option<TrianvertexNodeIndex>>,
    source: FixedDotIndex,
    target: FixedDotIndex,
}

#[derive(Error, Debug, Clone)]
pub enum NavmeshError {
    #[error("failed to insert vertex in navmesh")]
    Insertion(#[from] InsertionError),
}

impl Navmesh {
    pub fn new(
        layout: &Layout<impl RulesTrait>,
        source: FixedDotIndex,
        target: FixedDotIndex,
    ) -> Result<Self, NavmeshError> {
        let mut this = Self {
            triangulation: Triangulation::new(layout.drawing().geometry().graph().node_bound()),
            navvertex_to_trianvertex: Vec::new(),
            source,
            target,
        };
        this.navvertex_to_trianvertex
            .resize(layout.drawing().geometry().graph().node_bound(), None);

        let layer = layout.drawing().primitive(source).layer();
        let maybe_net = layout.drawing().primitive(source).maybe_net();

        for node in layout.drawing().layer_primitive_nodes(layer) {
            let primitive = node.primitive(layout.drawing());

            if let Some(primitive_net) = primitive.maybe_net() {
                if node == source.into()
                    || node == target.into()
                    || Some(primitive_net) != maybe_net
                {
                    match node {
                        PrimitiveIndex::FixedDot(dot) => {
                            this.triangulation.add_vertex(TrianvertexWeight {
                                node: dot.into(),
                                rails: vec![],
                                pos: primitive.shape().center(),
                            })?;
                        }
                        PrimitiveIndex::FixedBend(bend) => {
                            this.triangulation.add_vertex(TrianvertexWeight {
                                node: bend.into(),
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
                    this.navvertex_to_trianvertex[bend.petgraph_index().index()] =
                        Some(layout.drawing().primitive(bend).core().into());
                }
                _ => (),
            }
        }

        Ok(this)
    }

    fn trianvertex(&self, vertex: NavvertexNodeIndex) -> TrianvertexNodeIndex {
        match vertex {
            NavvertexNodeIndex::FixedDot(dot) => TrianvertexNodeIndex::FixedDot(dot),
            NavvertexNodeIndex::FixedBend(bend) => TrianvertexNodeIndex::FixedBend(bend),
            NavvertexNodeIndex::LooseBend(bend) => {
                self.navvertex_to_trianvertex[bend.petgraph_index().index()].unwrap()
            }
        }
    }

    pub fn source(&self) -> FixedDotIndex {
        self.source
    }

    pub fn target(&self) -> FixedDotIndex {
        self.target
    }
}

impl visit::GraphBase for Navmesh {
    type NodeId = NavvertexNodeIndex;
    type EdgeId = (NavvertexNodeIndex, NavvertexNodeIndex);
}

impl visit::Data for Navmesh {
    type NodeWeight = ();
    type EdgeWeight = ();
}

#[derive(Debug, Clone, Copy)]
pub struct NavmeshEdgeReference {
    from: NavvertexNodeIndex,
    to: NavvertexNodeIndex,
}

impl visit::EdgeRef for NavmeshEdgeReference {
    type NodeId = NavvertexNodeIndex;
    type EdgeId = (NavvertexNodeIndex, NavvertexNodeIndex);
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
    type Neighbors = Box<dyn Iterator<Item = NavvertexNodeIndex> + 'a>;

    fn neighbors(self, vertex: Self::NodeId) -> Self::Neighbors {
        Box::new(
            self.triangulation
                .neighbors(self.trianvertex(vertex))
                .flat_map(|neighbor| {
                    iter::once(neighbor.into()).chain(
                        self.triangulation
                            .weight(neighbor)
                            .rails
                            .iter()
                            .map(|index| NavvertexNodeIndex::from(*index)),
                    )
                }),
        )
    }
}

fn edge_with_near_edges(
    triangulation: &Triangulation<TrianvertexNodeIndex, TrianvertexWeight, ()>,
    edge: TriangulationEdgeReference<TrianvertexNodeIndex, ()>,
) -> impl Iterator<Item = NavmeshEdgeReference> {
    let mut from_vertices = vec![edge.source().into()];

    // Append rails to the source.
    from_vertices.extend(
        triangulation
            .weight(edge.source())
            .rails
            .iter()
            .map(|bend| NavvertexNodeIndex::from(*bend)),
    );

    let mut to_vertices = vec![edge.target().into()];

    // Append rails to the target.
    to_vertices.extend(
        triangulation
            .weight(edge.target())
            .rails
            .iter()
            .map(|bend| NavvertexNodeIndex::from(*bend)),
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
    triangulation: &Triangulation<TrianvertexNodeIndex, TrianvertexWeight, ()>,
    from: NavvertexNodeIndex,
    to: TrianvertexNodeIndex,
) -> impl Iterator<Item = NavmeshEdgeReference> {
    let from_vertices = vec![from];
    let mut to_vertices = vec![to.into()];

    // Append rails to the target.
    to_vertices.extend(
        triangulation
            .weight(to)
            .rails
            .iter()
            .map(|bend| NavvertexNodeIndex::from(*bend)),
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
                .edges(self.trianvertex(vertex))
                .flat_map(move |edge| vertex_edges(&self.triangulation, vertex, edge.target())),
        )
    }
}
