// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use pie::{
    navmesh::{self, EdgeIndex, TrianVertex},
    NavmeshIndex, RelaxedPath,
};
pub use planar_incr_embed as pie;

use geo::{Coord, LineString, Point};
use rstar::AABB;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use crate::{
    board::Board,
    drawing::{
        band::BandUid,
        bend::BendIndex,
        dot::{DotIndex, FixedDotIndex},
        graph::{MakePrimitive as _, PrimitiveIndex},
        head::{CaneHead, GetFace as _, Head},
        primitive::MakePrimitiveShape as _,
        rules::AccessRules,
        Collect as _,
    },
    geometry::{
        edit::ApplyGeometryEdit as _,
        primitive::PrimitiveShape,
        shape::{AccessShape as _, MeasureLength as _},
        GenericNode,
    },
    graph::GetPetgraphIndex as _,
    layout::{Layout, LayoutEdit},
    math::{CachedPolyExt, RotationSense},
    router::draw::{Draw, DrawException},
};

mod eval;
mod floating;
pub use floating::FloatingRouting;
mod poly;
use poly::*;
mod router;
pub use router::*;

#[derive(Clone, Copy, Debug)]
pub struct PieNavmeshBase;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize, serde::Serialize,
)]
pub struct EtchedPath {
    pub end_points: EdgeIndex<FixedDotIndex>,
}

impl EtchedPath {
    fn resolve_to_uid(
        &self,
        bands: &BTreeMap<EtchedPath, BandUid>,
    ) -> Result<BandUid, EvalException> {
        bands
            .get(self)
            .copied()
            .ok_or_else(|| EvalException::ResolvingPathFailed { path: *self })
    }
}

impl pie::NavmeshBase for PieNavmeshBase {
    type PrimalNodeIndex = FixedDotIndex;
    type EtchedPath = EtchedPath;
    type GapComment = ();
    type Scalar = f64;
}

pub type PieNavmesh = navmesh::Navmesh<PieNavmeshBase>;

pub type PieNavmeshRef<'a> = navmesh::NavmeshRef<'a, PieNavmeshBase>;

pub type PieEdgeIndex =
    EdgeIndex<NavmeshIndex<<PieNavmeshBase as pie::NavmeshBase>::PrimalNodeIndex>>;

/// Context for a single to-be-routed trace
#[derive(Clone, Debug)]
pub struct SubContext {
    pub label: EtchedPath,

    /// the last "active" head (head before the streak of `floating` entries)
    pub active_head: Head,

    pub polygon: Option<PolygonRouting>,

    // note that floating routing might be active while `poly` is also active,
    // in order to correctly calculate the exit points of the polygon.
    pub floating: Option<FloatingRouting>,
}

impl SubContext {
    pub fn is_end_point(&self, dot: FixedDotIndex) -> bool {
        dot == self.label.end_points[false] || dot == self.label.end_points[true]
    }
}

/// Data shared between many tasks
#[derive(Debug)]
pub struct Common<R> {
    pub layout: Layout<R>,

    pub active_layer: usize,

    /// width per path to be routed
    pub widths: BTreeMap<EtchedPath, f64>,

    /// If non-empty, then routing is only allowed to use these edges
    pub allowed_edges: BTreeSet<PieEdgeIndex>,
}

/// The context for [`PmgAstar`](pie::algo::pmg_astar::PmgAstar).
#[derive(Clone, Debug)]
pub struct AstarContext {
    /// TODO: make sure we can trust the `LayoutEdit`
    pub recorder: LayoutEdit,

    pub bands: BTreeMap<EtchedPath, BandUid>,

    /// length including `active_head`
    pub length: f64,

    pub sub: Option<SubContext>,
}

impl AstarContext {
    pub fn last_layout<R: AccessRules + Clone>(&self, common: &Common<R>) -> Layout<R> {
        let mut layout = common.layout.clone();
        layout.apply(&self.recorder);
        layout
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

impl core::ops::Neg for Alignment {
    type Output = Self;
    fn neg(self) -> Self {
        match self {
            Alignment::Left => Alignment::Right,
            Alignment::Center => Alignment::Center,
            Alignment::Right => Alignment::Left,
        }
    }
}

impl Alignment {
    /// ## Panics
    /// Panics if `self == Alignment::Right`
    pub fn incr_inplace(&mut self) {
        *self = match *self {
            Alignment::Left => Alignment::Center,
            Alignment::Center => Alignment::Right,
            Alignment::Right => panic!("too many alignment markers on edge"),
        }
    }
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum EvalException {
    #[error("floating routing exhausted, tunnel empty (origin = {origin:?})")]
    FloatingEmptyTunnel { origin: DotIndex },

    #[error("invalid polygon tangent arguments (origin = {origin:?})")]
    InvalidPolyTangentData {
        poly_ext: CachedPolyExt<FixedDotIndex>,
        origin: Point,
    },

    #[error("invalid polygon handover arguments")]
    InvalidPolyHandoverData {
        source_poly_ext: CachedPolyExt<FixedDotIndex>,
        source_sense: RotationSense,
        target_poly_ext: CachedPolyExt<FixedDotIndex>,
        target_sense: RotationSense,
    },

    #[error(transparent)]
    Draw(#[from] DrawException),

    #[error("unable to resolve path to BandUid")]
    ResolvingPathFailed { path: EtchedPath },

    #[error("route got bounced back")]
    RouteBouncedBack,

    #[error("route wrapped unnecessarily around end-point")]
    UnnecessaryWrapAroundEndpoint,

    #[error("inner path changed around polygon")]
    InnerPathChangedAroundPolygon {
        apex: FixedDotIndex,
        old_uid: Option<BandUid>,
        new_uid: Option<BandUid>,
    },

    #[error("bend not found")]
    BendNotFound { core: FixedDotIndex, uid: BandUid },

    #[error("edge disallowed: {0:?}")]
    EdgeDisallowed(PieEdgeIndex),

    #[error("panicked")]
    Panic(Arc<dyn std::any::Any + Send + 'static>),
}

impl EvalException {
    fn ghosts_blockers_and_obstacles(
        &self,
    ) -> (Vec<PrimitiveShape>, Vec<LineString>, Vec<PrimitiveIndex>) {
        match self {
            Self::FloatingEmptyTunnel { origin } => {
                (Vec::new(), Vec::new(), vec![(*origin).into()])
            }
            Self::InvalidPolyTangentData { poly_ext, .. } => (
                Vec::new(),
                vec![LineString(
                    poly_ext.0[..].iter().map(|&(pt, _, _)| pt.0).collect(),
                )],
                Vec::new(),
            ),
            Self::InvalidPolyHandoverData {
                source_poly_ext,
                target_poly_ext,
                ..
            } => (
                Vec::new(),
                vec![
                    LineString(
                        source_poly_ext.0[..]
                            .iter()
                            .map(|&(pt, _, _)| pt.0)
                            .collect(),
                    ),
                    LineString(
                        target_poly_ext.0[..]
                            .iter()
                            .map(|&(pt, _, _)| pt.0)
                            .collect(),
                    ),
                ],
                Vec::new(),
            ),
            Self::Draw(DrawException::NoTangents(_)) => (Vec::new(), Vec::new(), Vec::new()),
            Self::Draw(DrawException::CannotFinishIn(_, dwxc))
            | Self::Draw(DrawException::CannotWrapAround(_, dwxc)) => {
                match dwxc.maybe_ghost_and_obstacle() {
                    None => (Vec::new(), Vec::new(), Vec::new()),
                    Some((ghost, obstacle)) => (vec![*ghost], Vec::new(), vec![obstacle]),
                }
            }
            Self::ResolvingPathFailed { .. } => (Vec::new(), Vec::new(), Vec::new()),
            Self::RouteBouncedBack | Self::UnnecessaryWrapAroundEndpoint => {
                (Vec::new(), Vec::new(), Vec::new())
            }
            Self::InnerPathChangedAroundPolygon { .. } => (Vec::new(), Vec::new(), Vec::new()),
            Self::BendNotFound { .. } => (Vec::new(), Vec::new(), Vec::new()),
            Self::EdgeDisallowed(_) | Self::Panic(_) => (Vec::new(), Vec::new(), Vec::new()),
        }
    }
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum NavmeshCalculationError {
    #[error("Layer contains too few nodes to generate meaningful navmesh")]
    NotEnoughNodes,

    #[error("Unable to find boundary from node {node:?}, direction {direction:?}")]
    UnableToFindBoundary {
        node: FixedDotIndex,
        direction: Coord,
    },

    #[error(transparent)]
    Insertion(#[from] spade::InsertionError),
}

/// NOTE: this only works if the layer has ≥ 3 nodes
// TODO: handle the case with 2 nodes on the layer specifically.
pub fn calculate_navmesh<R: AccessRules>(
    board: &Board<R>,
    active_layer: usize,
) -> Result<PieNavmesh, NavmeshCalculationError> {
    use pie::NavmeshIndex::*;
    use spade::Triangulation;

    let triangulation = spade::DelaunayTriangulation::<TrianVertex<FixedDotIndex, f64>>::bulk_load(
        board
            .layout()
            .drawing()
            .rtree()
            .locate_in_envelope_intersecting(&AABB::<[f64; 3]>::from_corners(
                [-f64::INFINITY, -f64::INFINITY, active_layer as f64],
                [f64::INFINITY, f64::INFINITY, active_layer as f64],
            ))
            .map(|&geom| geom.data)
            .filter_map(|node| board.layout().apex_of_compoundless_node(node, active_layer))
            .map(|(idx, pos)| TrianVertex {
                idx,
                pos: spade::mitigate_underflow(spade::Point2 {
                    x: pos.x(),
                    y: pos.y(),
                }),
            })
            .collect(),
    )?;

    if triangulation.num_inner_faces() == 0 {
        log::warn!("calculate_navmesh: not enough nodes");
        return Err(NavmeshCalculationError::NotEnoughNodes);
    }

    let mut navmesh = navmesh::NavmeshSer::<PieNavmeshBase>::from_triangulation(&triangulation);

    let barrier2: Arc<[RelaxedPath<_, _>]> =
        Arc::from(vec![RelaxedPath::Weak(()), RelaxedPath::Weak(())]);

    let barrier0: Arc<[RelaxedPath<_, _>]> = Arc::from(vec![]);

    log::debug!("boundary = {:?}", board.layout().drawing().boundary());

    // populate Dual*-Dual* routed traces
    for (key, value) in &mut navmesh.edges {
        let wrap = |dot| GenericNode::Primitive(PrimitiveIndex::FixedDot(dot));
        match (value.0.lhs, value.0.rhs) {
            (Some(lhs), Some(rhs)) => {
                value.1 = barrier2.clone();
                let bands = board
                    .layout()
                    .bands_between_nodes_with_alignment(active_layer, wrap(lhs), wrap(rhs))
                    .map(|i| match i {
                        RelaxedPath::Weak(()) => RelaxedPath::Weak(()),
                        RelaxedPath::Normal(band_uid) => RelaxedPath::Normal(
                            *board.bands_by_id().get_by_right(&band_uid).unwrap(),
                        ),
                    })
                    .collect::<Vec<_>>();

                if bands != *barrier2 {
                    log::debug!("navmesh generated with {:?} = {:?}", value, &bands);
                    value.1 = Arc::from(bands);
                }
            }
            (None, Some(rhs)) => {
                value.1 = barrier0.clone();
                let direction = {
                    let (prev_key, next_key) = key.into();
                    let prev_dir = navmesh.nodes[prev_key]
                        .open_direction
                        .expect("expected DualOuter entry");
                    let next_dir = navmesh.nodes[next_key]
                        .open_direction
                        .expect("expected DualOuter entry");
                    Coord {
                        x: (prev_dir.x + next_dir.x) / 2.0,
                        y: (prev_dir.y + next_dir.y) / 2.0,
                    }
                };
                let bands = match board.layout().bands_between_node_and_boundary(
                    active_layer,
                    direction,
                    wrap(rhs),
                ) {
                    None => {
                        log::warn!("calculate_navmesh: unable to find boundary from node {:?}, direction {:?}", rhs, direction);
                        continue;
                        /*
                        return Err(NavmeshCalculationError::UnableToFindBoundary {
                            node: rhs,
                            direction,
                        });
                        */
                    }
                    Some(x) => {
                        x.map(|(band_uid, _)| {
                            RelaxedPath::Normal(
                                *board.bands_by_id().get_by_right(&band_uid).unwrap(),
                            )
                        })
                        .collect::<Vec<_>>()
                    }
                };

                if bands != *barrier0 {
                    log::debug!("navmesh generated with {:?} = {:?}", value, &bands);
                    value.1 = Arc::from(bands);
                }
            }
            (Some(lhs), None) => {
                value.1 = barrier0.clone();
                let direction = {
                    let (prev_key, next_key) = key.into();
                    let prev_dir = navmesh.nodes[prev_key]
                        .open_direction
                        .expect("expected DualOuter entry");
                    let next_dir = navmesh.nodes[next_key]
                        .open_direction
                        .expect("expected DualOuter entry");
                    Coord {
                        x: (prev_dir.x + next_dir.x) / 2.0,
                        y: (prev_dir.y + next_dir.y) / 2.0,
                    }
                };
                let mut bands = match board.layout().bands_between_node_and_boundary(
                    active_layer,
                    direction,
                    wrap(lhs),
                ) {
                    None => {
                        log::warn!("calculate_navmesh: unable to find boundary from node {:?}, direction {:?}", lhs, direction);
                        continue;
                        /*
                        return Err(NavmeshCalculationError::UnableToFindBoundary {
                            node: rhs,
                            direction,
                        });
                        */
                    }
                    Some(x) => {
                        x.map(|(band_uid, _)| {
                            RelaxedPath::Normal(
                                *board.bands_by_id().get_by_right(&band_uid).unwrap(),
                            )
                        })
                        .collect::<Vec<_>>()
                    }
                };
                bands.reverse();

                if bands != *barrier0 {
                    log::debug!("navmesh generated with {:?} = {:?}", value, &bands);
                    value.1 = Arc::from(bands);
                }
            }
            (None, None) => {
                // nothing to do
            }
        }
    }

    // TODO: insert fixed routed traces/bands into the navmesh

    // populate Primal-Dual* routed traces
    let dual_ends: BTreeMap<_, _> = navmesh
        .nodes
        .iter()
        .filter_map(|(node, data)| {
            if let Dual(node) = node {
                Some((node, data))
            } else {
                None
            }
        })
        .map(|(node, data)| {
            let mut binoccur = BTreeMap::new();
            for &neigh in &data.neighs {
                for &path in navmesh
                    .edge_data(Dual(*node), neigh)
                    .expect("unable to resolve neighbor")
                    .map::<_, RelaxedPath<FixedDotIndex, ()>, _>(|i| i[..].iter())
                {
                    if let RelaxedPath::Normal(ep) = path {
                        // every path should occur twice, or some other multiple of 2;
                        // find those which don't.
                        if binoccur.insert(ep, neigh).is_some() {
                            binoccur.remove(&ep);
                        }
                    }
                }
            }
            (*node, binoccur)
        })
        .filter(|(_, binoccur)| !binoccur.is_empty())
        .collect();

    let old_navmesh_edges_keys = navmesh.edges.keys().copied().collect::<Vec<_>>();

    // NOTE: this doesn't correctly handle the case that a path which ends in one dual node
    // occurs multiple times (we don't know which parts belong together)
    for key in old_navmesh_edges_keys {
        let (prim, dual) =
            if let (Primal(prim), Dual(dual)) | (Dual(dual), Primal(prim)) = key.into() {
                (prim, dual)
            } else {
                continue;
            };

        if let Some(dual_ends) = dual_ends.get(&dual) {
            for (&ep, &other) in dual_ends {
                // check if `ep` ends in `prim`.
                if !(ep.end_points[false] == prim || ep.end_points[true] == prim) {
                    continue;
                }

                // find ordering.
                let pos = navmesh.edges[&(Dual(dual), other).into()]
                    .1
                    .iter()
                    .position(|&i| i == RelaxedPath::Normal(ep))
                    .unwrap();
                match navmesh.planarr_find_other_end(&Dual(dual), &other, pos, true, &Primal(prim))
                {
                    None => {
                        log::warn!(
                            "topo-navmesh end path in planarr {:?}, {:?} -> {:?}: unable to find other end",
                            dual,
                            other,
                            prim,
                        );
                    }
                    Some((_, other_end)) => {
                        log::trace!(
                            "topo-navmesh end path in planarr {:?}, {:?} -> {:?}: other end @ {}",
                            dual,
                            other,
                            prim,
                            other_end.insert_pos,
                        );
                        // the edge is valid because it otherwise wouldn't have been the result from
                        // `planar_find_other_end` above
                        navmesh
                            .edge_data_mut(Dual(dual), Primal(prim))
                            .unwrap()
                            .with_borrow_mut(|mut x| {
                                x.insert(other_end.insert_pos, RelaxedPath::Normal(ep));
                            });
                    }
                }
            }
        }
    }

    Ok(navmesh.into())
}

impl SubContext {
    fn head_center<R: AccessRules>(&self, layout: &Layout<R>) -> Point {
        self.active_head
            .face()
            .primitive(layout.drawing())
            .shape()
            .center()
    }
}

fn cane_around<R: AccessRules>(
    layout: &mut Layout<R>,
    recorder: &mut LayoutEdit,
    route_length: &mut f64,
    old_head: Head,
    core: FixedDotIndex,
    inner: Option<BandUid>,
    sense: RotationSense,
    width: f64,
) -> Result<CaneHead, EvalException> {
    log::debug!(
        "cane around: head {:?}, core {:?}, inner {:?}, sense {:?}",
        old_head,
        core,
        inner,
        sense
    );

    let ret = match inner {
        None => layout.cane_around_dot(recorder, old_head, core, sense, width),
        Some(inner) => {
            // now, inner is expected to be a bend.
            // TODO: handle the case that the same path wraps multiple times around the same core
            let inner_bend = layout
                .drawing()
                .geometry()
                .all_rails(core.petgraph_index())
                .filter_map(|bi| {
                    if let BendIndex::Loose(lbi) = bi {
                        if layout.drawing().loose_band_uid(lbi.into()).ok() == Some(inner) {
                            Some(lbi)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .next();
            if let Some(inner_bend) = inner_bend {
                layout.cane_around_bend(recorder, old_head, inner_bend.into(), sense, width)
            } else {
                return Err(EvalException::BendNotFound {
                    core: core,
                    uid: inner,
                });
            }
        }
    }?;
    // record the length of the current seg, and the old bend, if any
    *route_length += ret.cane.seg.primitive(layout.drawing()).shape().length()
        + old_head
            .maybe_cane()
            .map(|cane| cane.bend.primitive(layout.drawing()).shape().length())
            .unwrap_or(0.0);
    Ok(ret)
}
