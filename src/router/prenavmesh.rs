// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_getters::Getters;
use enum_dispatch::enum_dispatch;
use geo::Point;
use petgraph::{stable_graph::NodeIndex, visit::NodeIndexable};
use spade::{HasPosition, InsertionError, Point2};

use crate::{
    drawing::{
        bend::FixedBendIndex,
        dot::{DotIndex, FixedDotIndex},
        graph::{GetMaybeNet, MakePrimitive, PrimitiveIndex},
        primitive::{GetCore, GetJoints, MakePrimitiveShape, Primitive},
        rules::AccessRules,
        seg::{FixedSegIndex, LoneLooseSegIndex, SeqLooseSegIndex},
        Drawing,
    },
    geometry::{shape::AccessShape, GetLayer},
    graph::GetPetgraphIndex,
    layout::Layout,
    triangulation::{GetTrianvertexNodeIndex, Triangulation},
};

use super::{navmesh::NavmeshError, RouterOptions};

/// Prenavmesh nodes are the vertices of constrained Delaunay triangulation
/// before it is converted to the navmesh, which is done by multiplying each
/// of the prenavmesh nodes into more nodes, called navnodes.
#[enum_dispatch(GetPetgraphIndex, MakePrimitive)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PrenavmeshNodeIndex {
    FixedDot(FixedDotIndex),
    FixedBend(FixedBendIndex),
}

#[derive(Clone, Copy)]
pub struct PrenavmeshWeight {
    pub node: PrenavmeshNodeIndex,
    pub pos: Point,
}

impl GetTrianvertexNodeIndex<PrenavmeshNodeIndex> for PrenavmeshWeight {
    fn node_index(&self) -> PrenavmeshNodeIndex {
        self.node
    }
}

impl HasPosition for PrenavmeshWeight {
    type Scalar = f64;
    fn position(&self) -> Point2<Self::Scalar> {
        Point2::new(self.pos.x(), self.pos.y())
    }
}

impl PrenavmeshWeight {
    pub fn new_from_fixed_dot(layout: &Layout<impl AccessRules>, dot: FixedDotIndex) -> Self {
        Self {
            node: dot.into(),
            pos: dot.primitive(layout.drawing()).shape().center(),
        }
    }

    pub fn new_from_fixed_bend(layout: &Layout<impl AccessRules>, bend: FixedBendIndex) -> Self {
        Self {
            node: bend.into(),
            pos: bend.primitive(layout.drawing()).shape().center(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct PrenavmeshConstraint(pub PrenavmeshWeight, pub PrenavmeshWeight);

impl PrenavmeshConstraint {
    pub fn new_from_fixed_dot_pair(
        layout: &Layout<impl AccessRules>,
        from_dot: FixedDotIndex,
        to_dot: FixedDotIndex,
    ) -> Self {
        Self(
            PrenavmeshWeight::new_from_fixed_dot(layout, from_dot),
            PrenavmeshWeight::new_from_fixed_dot(layout, to_dot),
        )
    }

    pub fn new_from_lone_loose_seg(
        layout: &Layout<impl AccessRules>,
        seg: LoneLooseSegIndex,
    ) -> Self {
        let (from_dot, to_dot) = layout.drawing().primitive(seg).joints();
        Self::new_from_fixed_dot_pair(layout, from_dot, to_dot)
    }

    pub fn new_from_seq_loose_seg(
        layout: &Layout<impl AccessRules>,
        seg: SeqLooseSegIndex,
    ) -> Self {
        let (from_joint, to_joint) = layout.drawing().primitive(seg).joints();

        let from_dot = match from_joint {
            DotIndex::Fixed(dot) => dot,
            DotIndex::Loose(dot) => {
                let bend = layout.drawing().primitive(dot).bend();

                layout.drawing().primitive(bend).core()
            }
        };

        let to_bend = layout.drawing().primitive(to_joint).bend();
        let to_dot = layout.drawing().primitive(to_bend).core();
        Self::new_from_fixed_dot_pair(layout, from_dot, to_dot)
    }

    pub fn new_from_fixed_seg(layout: &Layout<impl AccessRules>, seg: FixedSegIndex) -> Self {
        let (from_dot, to_dot) = layout.drawing().primitive(seg).joints();
        Self::new_from_fixed_dot_pair(layout, from_dot, to_dot)
    }
}

#[derive(Clone, Getters)]
pub struct Prenavmesh {
    triangulation: Triangulation<PrenavmeshNodeIndex, PrenavmeshWeight, ()>,
    constraints: Vec<PrenavmeshConstraint>,
}

impl Prenavmesh {
    pub fn new(
        layout: &Layout<impl AccessRules>,
        origin: FixedDotIndex,
        destination: FixedDotIndex,
        _options: RouterOptions,
    ) -> Result<Self, NavmeshError> {
        let mut this = Self {
            triangulation: Triangulation::new(layout.drawing().geometry().graph().node_bound()),
            constraints: vec![],
        };

        let layer = layout.drawing().primitive(origin).layer();
        let maybe_net = layout.drawing().primitive(origin).maybe_net();

        for node in layout.drawing().layer_primitive_nodes(layer) {
            let primitive = node.primitive(layout.drawing());

            if let Some(primitive_net) = primitive.maybe_net() {
                if node == origin.into()
                    || node == destination.into()
                    || Some(primitive_net) != maybe_net
                {
                    match node {
                        PrimitiveIndex::FixedDot(dot) => {
                            this.triangulation
                                .add_vertex(PrenavmeshWeight::new_from_fixed_dot(layout, dot))?;
                        }
                        PrimitiveIndex::LoneLooseSeg(seg) => {
                            this.add_constraint(PrenavmeshConstraint::new_from_lone_loose_seg(
                                layout, seg,
                            ))?;
                        }
                        PrimitiveIndex::SeqLooseSeg(seg) => {
                            this.add_constraint(PrenavmeshConstraint::new_from_seq_loose_seg(
                                layout, seg,
                            ))?;
                        }
                        PrimitiveIndex::FixedBend(bend) => {
                            this.triangulation
                                .add_vertex(PrenavmeshWeight::new_from_fixed_bend(layout, bend))?;
                        }
                        _ => (),
                    }
                }
            }
        }

        for node in layout.drawing().layer_primitive_nodes(layer) {
            let primitive = node.primitive(layout.drawing());

            if let Some(primitive_net) = primitive.maybe_net() {
                if node == origin.into()
                    || node == destination.into()
                    || Some(primitive_net) != maybe_net
                {
                    // If you have a band that was routed from a polygonal pad,
                    // when you will start a new routing some of the constraint
                    // edges created from the loose segs of a band will
                    // intersect some of the constraint edges created from the
                    // fixed segs constituting the pad boundary.
                    //
                    // Such constraint intersections are erroneous and cause
                    // Spade to throw a panic at runtime. So, to prevent this
                    // from occuring, we iterate over the layout for the second
                    // time, after all the constraint edges from bands have been
                    // placed, and only then add constraint edges created from
                    // fixed segs that do not cause an intersection.
                    match node {
                        PrimitiveIndex::FixedSeg(seg) => {
                            let constraint = PrenavmeshConstraint::new_from_fixed_seg(layout, seg);

                            if !this
                                .triangulation
                                .intersects_constraint(&constraint.0, &constraint.1)
                            {
                                this.add_constraint(constraint);
                            }
                        }
                        _ => (),
                    }
                }
            }
        }

        Ok(this)
    }

    fn add_constraint(&mut self, constraint: PrenavmeshConstraint) -> Result<(), InsertionError> {
        self.triangulation
            .add_constraint_edge(constraint.0, constraint.1)?;
        self.constraints.push(constraint);
        Ok(())
    }
}
