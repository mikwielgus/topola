// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use core::ops::ControlFlow;
use geo::geometry::LineString;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use crate::{
    autorouter::invoker::GetDebugOverlayData,
    board::edit::BoardEdit,
    drawing::{band::BandUid, dot::FixedDotIndex, graph::PrimitiveIndex, rules::AccessRules},
    geometry::primitive::PrimitiveShape,
    graph::GenericIndex,
    layout::{poly::PolyWeight, Layout},
    stepper::{Abort, EstimateProgress},
};

use super::{
    pie::{
        self,
        algo::{pmg_astar::InsertionInfo, Goal as PieGoal},
        navmesh::{EdgeIndex, EdgePaths},
        Edge, NavmeshIndex, Node, RelaxedPath,
    },
    AstarContext, Common, EtchedPath, PieEdgeIndex, PieNavmesh, PieNavmeshBase,
};

pub type PmgAstar = pie::algo::pmg_astar::PmgAstar<PieNavmeshBase, AstarContext>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Goal {
    pub source: FixedDotIndex,
    pub target: FixedDotIndex,
    pub width: f64,
}

impl Goal {
    #[inline]
    pub fn label(&self) -> EtchedPath {
        EtchedPath {
            end_points: EdgeIndex::from((self.source, self.target)),
        }
    }

    pub fn to_pie(&self) -> PieGoal<FixedDotIndex, EtchedPath> {
        assert_ne!(self.source, self.target);
        let mut target = BTreeSet::new();
        target.insert(self.target);
        PieGoal {
            source: self.source,
            target,
            label: self.label(),
        }
    }

    pub fn register<R>(&self, common: &mut Common<R>) {
        common.widths.insert(self.label(), self.width);
    }
}

/// Manages the autorouting process across multiple ratlines.
pub struct AutorouteExecutionStepper<R> {
    pmg_astar: PmgAstar,
    common: Common<R>,
    original_edge_paths: Box<[EdgePaths<EtchedPath, ()>]>,

    pub last_layout: Layout<R>,
    pub last_recorder: BoardEdit,
    pub last_edge_paths: Box<[EdgePaths<EtchedPath, ()>]>,
    pub last_bands: BTreeMap<EtchedPath, BandUid>,

    pub active_polygons: Vec<GenericIndex<PolyWeight>>,
    pub ghosts: Vec<PrimitiveShape>,
    pub polygonal_blockers: Vec<LineString>,
    pub obstacles: Vec<PrimitiveIndex>,
}

impl<R: Clone> AutorouteExecutionStepper<R> {
    fn finish(&mut self) {
        self.active_polygons.clear();
        self.ghosts.clear();
        self.obstacles.clear();
        self.polygonal_blockers.clear();
    }
}

impl<M: Clone + std::panic::RefUnwindSafe> Abort<()> for AutorouteExecutionStepper<M> {
    fn abort(&mut self, _: &mut ()) {
        self.last_layout = self.common.layout.clone();
        self.last_recorder = BoardEdit::new();
        self.last_edge_paths = self.original_edge_paths.clone();
        self.last_bands = BTreeMap::new();
        self.finish();
    }
}

impl<R: AccessRules + Clone + std::panic::RefUnwindSafe> AutorouteExecutionStepper<R> {
    /// `navmesh` can be initialized using [`calculate_navmesh`](super::calculate_navmesh)
    pub fn new(
        layout: &Layout<R>,
        navmesh: &PieNavmesh,
        bands: BTreeMap<EtchedPath, BandUid>,
        active_layer: usize,
        allowed_edges: BTreeSet<PieEdgeIndex>,
        goals: impl Iterator<Item = Goal>,
    ) -> Self {
        let mut common = Common {
            layout: layout.clone(),
            active_layer,
            widths: BTreeMap::new(),
            allowed_edges,
        };

        let mut astar_goals = Vec::new();
        for i in goals {
            i.register(&mut common);
            astar_goals.push(i.to_pie());
        }

        let context = AstarContext {
            recorder: BoardEdit::new(),
            bands,
            length: 0.0,
            sub: None,
        };

        let (mut ghosts, mut polygonal_blockers, mut obstacles) =
            (Vec::new(), Vec::new(), Vec::new());

        let pmg_astar = PmgAstar::new(
            navmesh,
            astar_goals,
            &context,
            |navmesh, context, ins_info| match AstarContext::evaluate_navmesh(
                navmesh, context, &common, ins_info,
            ) {
                Ok(x) => Some(x),
                Err(e) => {
                    let (mut new_ghosts, mut new_blockers, mut new_obstacles) =
                        e.ghosts_blockers_and_obstacles();
                    ghosts.append(&mut new_ghosts);
                    polygonal_blockers.append(&mut new_blockers);
                    obstacles.append(&mut new_obstacles);
                    None
                }
            },
        );

        Self {
            pmg_astar,
            common,
            original_edge_paths: navmesh.edge_paths.clone(),
            last_layout: layout.clone(),
            last_recorder: BoardEdit::new(),
            last_edge_paths: navmesh.edge_paths.clone(),
            last_bands: context.bands.clone(),
            active_polygons: Vec::new(),
            ghosts,
            polygonal_blockers,
            obstacles,
        }
    }

    pub fn step(&mut self) -> ControlFlow<bool, ()> {
        self.active_polygons.clear();
        self.ghosts.clear();
        self.obstacles.clear();
        self.polygonal_blockers.clear();

        match self.pmg_astar.step(
            |navmesh, context, ins_info| match AstarContext::evaluate_navmesh(
                navmesh,
                context,
                &self.common,
                ins_info,
            ) {
                Ok(x) => {
                    if let Some(poly) = x.1.sub.as_ref().and_then(|i| i.polygon.as_ref()) {
                        if !self.active_polygons.contains(&poly.idx) {
                            self.active_polygons.push(poly.idx);
                        }
                    }
                    Some(x)
                }
                Err(e) => {
                    log::debug!("eval-navmesh error: {:?}", e);
                    let (mut new_ghosts, mut new_blockers, mut new_obstacles) =
                        e.ghosts_blockers_and_obstacles();
                    if let Some(poly) = context.sub.as_ref().and_then(|i| i.polygon.as_ref()) {
                        if !self.active_polygons.contains(&poly.idx) {
                            self.active_polygons.push(poly.idx);
                        }
                    }
                    self.ghosts.append(&mut new_ghosts);
                    self.polygonal_blockers.append(&mut new_blockers);
                    self.obstacles.append(&mut new_obstacles);
                    None
                }
            },
        ) {
            // no valid result found
            ControlFlow::Break(None) => {
                self.last_layout = self.common.layout.clone();
                self.last_recorder = BoardEdit::new();
                self.last_edge_paths = self.original_edge_paths.clone();
                self.last_bands = BTreeMap::new();
                self.finish();
                ControlFlow::Break(false)
            }

            // some valid result found
            ControlFlow::Break(Some((_costs, edge_paths, ctx))) => {
                self.last_layout = ctx.last_layout(&self.common);
                self.last_recorder = ctx.recorder;
                self.last_edge_paths = edge_paths;
                self.last_bands = ctx.bands;
                self.finish();
                ControlFlow::Break(true)
            }

            // intermediate data
            ControlFlow::Continue(intermed) => {
                self.last_layout = intermed.context.last_layout(&self.common);
                self.last_edge_paths = intermed.edge_paths;

                ControlFlow::Continue(())
            }
        }
    }
}

impl<M> EstimateProgress for AutorouteExecutionStepper<M> {
    type Value = f64;
}

impl<M> GetDebugOverlayData for AutorouteExecutionStepper<M> {
    fn maybe_topo_navmesh(&self) -> Option<pie::navmesh::NavmeshRef<'_, super::PieNavmeshBase>> {
        Some(pie::navmesh::NavmeshRef {
            nodes: &self.pmg_astar.nodes,
            edges: &self.pmg_astar.edges,
            edge_paths: &self.last_edge_paths,
        })
    }

    fn active_polygons(&self) -> &[GenericIndex<PolyWeight>] {
        &self.active_polygons[..]
    }

    fn ghosts(&self) -> &[PrimitiveShape] {
        &self.ghosts[..]
    }

    fn obstacles(&self) -> &[PrimitiveIndex] {
        &self.obstacles[..]
    }

    fn polygonal_blockers(&self) -> &[LineString] {
        &self.polygonal_blockers[..]
    }
}

#[derive(Clone, Debug)]
struct ManualRouteStep {
    goal: Goal,
    edge_paths: Box<[EdgePaths<EtchedPath, ()>]>,
    prev_node: Option<NavmeshIndex<FixedDotIndex>>,
    cur_node: NavmeshIndex<FixedDotIndex>,
}

/// Manages the manual "etching"/"implementation" of lowering paths from a topological navmesh
/// onto a layout.
pub struct ManualrouteExecutionStepper<R> {
    /// remaining steps are in reverse order
    remaining_steps: Vec<ManualRouteStep>,
    context: AstarContext,
    aborted: bool,

    // constant stuff
    nodes: Arc<BTreeMap<NavmeshIndex<FixedDotIndex>, Node<FixedDotIndex, f64>>>,
    edges: Arc<BTreeMap<EdgeIndex<NavmeshIndex<FixedDotIndex>>, (Edge<FixedDotIndex>, usize)>>,
    common: Common<R>,
    original_edge_paths: Box<[EdgePaths<EtchedPath, ()>]>,

    // results
    pub last_layout: Layout<R>,
    pub last_recorder: BoardEdit,

    // visualization / debug
    pub active_polygons: Vec<GenericIndex<PolyWeight>>,
    pub ghosts: Vec<PrimitiveShape>,
    pub polygonal_blockers: Vec<LineString>,
    pub obstacles: Vec<PrimitiveIndex>,
}

impl<M> ManualrouteExecutionStepper<M> {
    pub fn last_edge_paths(&self) -> &Box<[EdgePaths<EtchedPath, ()>]> {
        match self.remaining_steps.last() {
            _ if self.aborted => &self.original_edge_paths,
            Some(goal) => &goal.edge_paths,
            None => &self.original_edge_paths,
        }
    }

    pub fn last_bands(&self) -> &BTreeMap<EtchedPath, BandUid> {
        &self.context.bands
    }
}

impl<M: Clone> ManualrouteExecutionStepper<M> {
    fn finish(&mut self) {
        self.active_polygons.clear();
        self.ghosts.clear();
        self.obstacles.clear();
        self.polygonal_blockers.clear();
    }
}

impl<M: Clone + std::panic::RefUnwindSafe> Abort<()> for ManualrouteExecutionStepper<M> {
    fn abort(&mut self, _: &mut ()) {
        self.last_layout = self.common.layout.clone();
        self.last_recorder = BoardEdit::new();
        self.context.bands = BTreeMap::new();
        self.finish();
        self.aborted = true;
    }
}

impl<R: AccessRules + Clone + std::panic::RefUnwindSafe> ManualrouteExecutionStepper<R> {
    /// `navmesh` can be initialized using [`calculate_navmesh`](super::calculate_navmesh).
    /// This function assumes that `dest_navmesh` is already populated with all the `goals`, but `layout` isn't.
    ///
    /// ## Panics
    /// This function panics if the goals mention paths which don't exist in the navmesh,
    /// or if the original navmesh is inconsistent.
    pub fn new(
        layout: &Layout<R>,
        dest_navmesh: &PieNavmesh,
        bands: BTreeMap<EtchedPath, BandUid>,
        active_layer: usize,
        goals: impl core::iter::DoubleEndedIterator<Item = Goal>,
    ) -> Self {
        let mut common = Common {
            layout: layout.clone(),
            active_layer,
            widths: BTreeMap::new(),
            allowed_edges: BTreeSet::new(),
        };

        let nodes = dest_navmesh.nodes.clone();
        let edges = dest_navmesh.edges.clone();
        let mut mres_steps = Vec::new();
        let mut tmp_navmesh_edge_paths = dest_navmesh.edge_paths.clone();
        for i in goals.rev() {
            i.register(&mut common);
            mres_steps.push(ManualRouteStep {
                goal: i,
                edge_paths: tmp_navmesh_edge_paths.clone(),
                prev_node: None,
                cur_node: NavmeshIndex::Primal(i.source),
            });
            pie::navmesh::remove_path(&mut tmp_navmesh_edge_paths, &RelaxedPath::Normal(i.label()));
        }

        let context = AstarContext {
            recorder: BoardEdit::new(),
            bands,
            length: 0.0,
            sub: None,
        };

        Self {
            remaining_steps: mres_steps,
            context,
            aborted: false,

            nodes,
            edges,
            common,
            original_edge_paths: tmp_navmesh_edge_paths,

            last_layout: layout.clone(),
            last_recorder: BoardEdit::new(),
            active_polygons: Vec::new(),
            ghosts: Vec::new(),
            polygonal_blockers: Vec::new(),
            obstacles: Vec::new(),
        }
    }

    /// This function might panic if some goal's route loops back to where it came from
    pub fn step(&mut self) -> ControlFlow<bool, ()> {
        self.active_polygons.clear();
        self.ghosts.clear();
        self.obstacles.clear();
        self.polygonal_blockers.clear();

        let goal = if let Some(goal) = self.remaining_steps.last_mut() {
            goal
        } else {
            // some valid result found
            self.last_layout = self.context.last_layout(&self.common);
            self.last_recorder = self.context.recorder.clone();
            self.finish();
            return ControlFlow::Break(true);
        };

        // try to advance current goal
        let source = goal.cur_node.clone();
        let label = goal.goal.label();
        let navmesh = pie::navmesh::NavmeshRef::<'_, super::PieNavmeshBase> {
            nodes: &*self.nodes,
            edges: &*self.edges,
            edge_paths: &goal.edge_paths,
        };

        let ins_info = match goal.prev_node {
            None => {
                // bootstrap this goal (see also pie's `start_pmga`)
                navmesh
                    .node_data(&source)
                    .unwrap()
                    .neighs
                    .iter()
                    .find_map(|&neigh| {
                        // `edge_data`: can't panic because the original navmesh is valid (assumed to be)
                        // and nodes, edges stay the same.
                        let (edge_meta, epi) = navmesh.resolve_edge_data(source, neigh).unwrap();
                        navmesh
                            .access_edge_paths(epi)
                            .iter()
                            .enumerate()
                            .find(|&(_, j)| j == &RelaxedPath::Normal(label))
                            .map(|(intro, _)| InsertionInfo {
                                prev_node: source,
                                cur_node: neigh,
                                edge_meta: edge_meta.to_owned(),
                                epi,
                                intro,
                                maybe_new_goal: Some(label),
                            })
                    })
                    .unwrap()
            }
            Some(_) if source == NavmeshIndex::Primal(goal.goal.target) => {
                // this goal is finished -> pursue next goal
                let edge_paths = goal.edge_paths.clone();
                self.remaining_steps.pop();
                if self.remaining_steps.is_empty() {
                    self.original_edge_paths = edge_paths;
                }
                return ControlFlow::Continue(());
            }
            Some(prev_node) => {
                // find next edge
                navmesh
                    .node_data(&source)
                    .unwrap()
                    .neighs
                    .iter()
                    // we never want to go back to where we came from
                    .filter(|&neigh| *neigh != prev_node)
                    .find_map(|&neigh| {
                        // `edge_data`: can't panic because the original navmesh is valid (assumed to be)
                        // and nodes, edges stay the same.
                        let (edge_meta, epi) = navmesh.resolve_edge_data(source, neigh).unwrap();
                        navmesh
                            .access_edge_paths(epi)
                            .iter()
                            .enumerate()
                            .find(|&(_, j)| j == &RelaxedPath::Normal(label))
                            .map(|(intro, _)| InsertionInfo {
                                prev_node: source,
                                cur_node: neigh,
                                edge_meta: edge_meta.to_owned(),
                                epi,
                                intro,
                                maybe_new_goal: None,
                            })
                    })
                    .unwrap()
            }
        };

        goal.prev_node = Some(ins_info.prev_node);
        goal.cur_node = ins_info.cur_node;

        match AstarContext::evaluate_navmesh(navmesh, &self.context, &self.common, ins_info) {
            Ok((_costs, context)) => {
                if let Some(poly) = context.sub.as_ref().and_then(|i| i.polygon.as_ref()) {
                    if !self.active_polygons.contains(&poly.idx) {
                        self.active_polygons.push(poly.idx);
                    }
                }

                self.context = context;
                self.last_layout = self.context.last_layout(&self.common);
                ControlFlow::Continue(())
            }
            Err(e) => {
                log::debug!("eval-navmesh error: {:?}", e);
                let (mut new_ghosts, mut new_blockers, mut new_obstacles) =
                    e.ghosts_blockers_and_obstacles();
                if let Some(poly) = self.context.sub.as_ref().and_then(|i| i.polygon.as_ref()) {
                    if !self.active_polygons.contains(&poly.idx) {
                        self.active_polygons.push(poly.idx);
                    }
                }
                self.ghosts.append(&mut new_ghosts);
                self.polygonal_blockers.append(&mut new_blockers);
                self.obstacles.append(&mut new_obstacles);

                // no valid result found
                self.last_layout = self.common.layout.clone();
                self.last_recorder = BoardEdit::new();
                self.context.bands = BTreeMap::new();
                self.finish();
                ControlFlow::Break(false)
            }
        }
    }
}

impl<M> GetDebugOverlayData for ManualrouteExecutionStepper<M> {
    fn maybe_topo_navmesh(&self) -> Option<pie::navmesh::NavmeshRef<'_, super::PieNavmeshBase>> {
        Some(pie::navmesh::NavmeshRef {
            nodes: &self.nodes,
            edges: &self.edges,
            edge_paths: &self.last_edge_paths(),
        })
    }

    fn active_polygons(&self) -> &[GenericIndex<PolyWeight>] {
        &self.active_polygons[..]
    }

    fn ghosts(&self) -> &[PrimitiveShape] {
        &self.ghosts[..]
    }

    fn obstacles(&self) -> &[PrimitiveIndex] {
        &self.obstacles[..]
    }

    fn polygonal_blockers(&self) -> &[LineString] {
        &self.polygonal_blockers[..]
    }
}
