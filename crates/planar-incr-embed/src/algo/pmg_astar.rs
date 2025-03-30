// SPDX-FileCopyrightText: 2025 Topola contributors
// SPDX-FileCopyrightText: 2021 petgraph contributors
//
// SPDX-License-Identifier: MIT
//
//! planar multi-goal A*-like path search implementation

use crate::{
    algo::{Goal, PreparedGoal},
    mayrev::MaybeReversed,
    navmesh::{EdgeIndex, EdgePaths, Navmesh, NavmeshRef, NavmeshRefMut},
    Edge, NavmeshBase, NavmeshIndex, RelaxedPath,
};

use alloc::collections::{BTreeMap, BinaryHeap};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::{cmp::Ordering, ops::ControlFlow};
use num_traits::float::TotalOrder;

/// A walk task
#[derive(Clone, Debug)]
pub struct Task<B: NavmeshBase, Ctx> {
    /// index of current goal
    pub goal_idx: usize,

    /// costs / weights accumulated so far
    pub costs: B::Scalar,

    /// estimated minimal costs until goal
    pub estimated_remaining: B::Scalar,

    /// estimated minimal costs for all remaining goals after this one
    pub estimated_remaining_goals: B::Scalar,

    /// the current navmesh edge paths
    pub edge_paths: Box<[EdgePaths<B::EtchedPath, B::GapComment>]>,

    /// the currently selected node
    pub selected_node: NavmeshIndex<B::PrimalNodeIndex>,

    /// the previously selected node
    pub prev_node: NavmeshIndex<B::PrimalNodeIndex>,

    /// the introduction position re: `selected_node`
    pub cur_intro: usize,

    /// associated context (ignored during comparisons)
    pub context: Ctx,
}

/// Results after a [`Task`] is done.
#[derive(Clone, Debug)]
pub struct TaskResult<B: NavmeshBase, Ctx> {
    /// index of current goal
    pub goal_idx: usize,

    /// costs / weights accumulated so far
    pub costs: B::Scalar,

    /// the current navmesh edges
    pub edge_paths: Box<[EdgePaths<B::EtchedPath, B::GapComment>]>,

    /// the previously selected node
    pub prev_node: NavmeshIndex<B::PrimalNodeIndex>,

    /// the introduction position re: `target`
    pub cur_intro: usize,

    /// the associated context
    pub context: Ctx,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InsertionInfo<B: NavmeshBase> {
    pub prev_node: NavmeshIndex<B::PrimalNodeIndex>,
    pub cur_node: NavmeshIndex<B::PrimalNodeIndex>,
    pub edge_meta: Edge<B::PrimalNodeIndex>,
    pub epi: MaybeReversed<usize, RelaxedPath<B::EtchedPath, B::GapComment>>,

    /// the introduction position re: `prev_node` -edge-> `cur_node`
    pub intro: usize,

    pub maybe_new_goal: Option<B::EtchedPath>,
}

pub trait EvaluateNavmesh<B: NavmeshBase, Ctx>:
    FnMut(NavmeshRef<B>, &Ctx, InsertionInfo<B>) -> Option<(B::Scalar, Ctx)>
{
}

impl<B, Ctx, F> EvaluateNavmesh<B, Ctx> for F
where
    B: NavmeshBase,
    F: FnMut(NavmeshRef<B>, &Ctx, InsertionInfo<B>) -> Option<(B::Scalar, Ctx)>,
{
}

/// The main path search data structure
#[derive(Clone, Debug)]
pub struct PmgAstar<B: NavmeshBase, Ctx> {
    /// task queue, ordered by costs ascending
    pub queue: BinaryHeap<Task<B, Ctx>>,

    // constant data
    pub nodes:
        Arc<BTreeMap<NavmeshIndex<B::PrimalNodeIndex>, crate::Node<B::PrimalNodeIndex, B::Scalar>>>,
    pub edges: Arc<
        BTreeMap<EdgeIndex<NavmeshIndex<B::PrimalNodeIndex>>, (Edge<B::PrimalNodeIndex>, usize)>,
    >,
    pub goals: Box<[PreparedGoal<B>]>,
}

impl<B: NavmeshBase, Ctx> Task<B, Ctx>
where
    B::Scalar: num_traits::Float,
{
    fn edge_paths_count(&self) -> usize {
        self.edge_paths.iter().map(|i| i.len()).sum::<usize>()
    }

    fn estimated_full_costs(&self) -> B::Scalar {
        self.costs + self.estimated_remaining + self.estimated_remaining_goals
    }
}

impl<B: NavmeshBase, Ctx> PartialEq for Task<B, Ctx>
where
    B::PrimalNodeIndex: Ord,
    B::EtchedPath: PartialOrd,
    B::GapComment: PartialOrd,
    B::Scalar: num_traits::Float + num_traits::float::TotalOrder + PartialOrd,
{
    fn eq(&self, other: &Self) -> bool {
        self.estimated_full_costs()
            .total_cmp(&other.estimated_full_costs())
            == Ordering::Equal
            && self.goal_idx == other.goal_idx
            && other.edge_paths_count() == self.edge_paths_count()
            && self.selected_node == other.selected_node
            && self.prev_node == other.prev_node
            && self.cur_intro == other.cur_intro
            && self
                .edge_paths
                .partial_cmp(&other.edge_paths)
                .map(|i| i == Ordering::Equal)
                .unwrap_or(true)
    }
}

impl<B: NavmeshBase, Ctx> Eq for Task<B, Ctx>
where
    B::PrimalNodeIndex: Ord,
    B::EtchedPath: PartialOrd,
    B::GapComment: PartialOrd,
    B::Scalar: num_traits::Float + num_traits::float::TotalOrder + PartialOrd,
{
}

// tasks are ordered such that smaller costs and higher goal indices are ordered as being larger (better)
impl<B: NavmeshBase, Ctx> Ord for Task<B, Ctx>
where
    B::PrimalNodeIndex: Ord,
    B::EtchedPath: PartialOrd,
    B::GapComment: PartialOrd,
    B::Scalar: num_traits::Float + num_traits::float::TotalOrder + PartialOrd,
{
    fn cmp(&self, other: &Self) -> Ordering {
        // smaller costs are better
        other
            .estimated_full_costs()
            .total_cmp(&self.estimated_full_costs())
            // higher goal index is better
            .then_with(|| self.goal_idx.cmp(&other.goal_idx))
            // less inserted paths in edges are better
            .then_with(|| other.edge_paths_count().cmp(&self.edge_paths_count()))
            // tie-break on the rest
            .then_with(|| self.selected_node.cmp(&other.selected_node))
            .then_with(|| self.prev_node.cmp(&other.prev_node))
            .then_with(|| self.cur_intro.cmp(&other.cur_intro))
            .then_with(|| {
                self.edge_paths
                    .partial_cmp(&other.edge_paths)
                    .unwrap_or(Ordering::Equal)
            })
    }
}

impl<B: NavmeshBase, Ctx> PartialOrd for Task<B, Ctx>
where
    B::PrimalNodeIndex: Ord,
    B::EtchedPath: PartialOrd,
    B::GapComment: PartialOrd,
    B::Scalar: num_traits::Float + num_traits::float::TotalOrder + PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<B: NavmeshBase<Scalar = Scalar>, Scalar: num_traits::Float + core::iter::Sum, Ctx>
    PmgAstar<B, Ctx>
{
    fn estimate_remaining_goals_costs(&self, start_goal_idx: usize) -> Scalar {
        self.goals
            .get(start_goal_idx + 1..)
            .map(|rgoals| rgoals.iter().map(|i| i.minimal_costs).sum())
            .unwrap_or_else(Scalar::zero)
    }
}

impl<B: NavmeshBase> PreparedGoal<B>
where
    B::EtchedPath: PartialOrd,
    B::GapComment: PartialOrd + Clone,
    B::Scalar: num_traits::Float + num_traits::float::TotalOrder + core::iter::Sum,
{
    /// start processing the goal
    fn start_pmga<Ctx, F: EvaluateNavmesh<B, Ctx>>(
        &self,
        navmesh: NavmeshRef<'_, B>,
        goal_idx: usize,
        env: &PmgAstar<B, Ctx>,
        context: &Ctx,
        evaluate_navmesh: &mut F,
    ) -> BinaryHeap<Task<B, Ctx>> {
        let source = NavmeshIndex::Primal(self.source.clone());
        let estimated_remaining_goals = env.estimate_remaining_goals_costs(goal_idx);
        let neighs = match navmesh.node_data(&source) {
            None => return BinaryHeap::new(),
            Some(x) => &x.neighs,
        };

        let mut ret = BinaryHeap::new();

        // NOTE: this uses a `for` loop to get around borrowing problems with `evaluate_navmesh`
        for (neigh, emeta, epi, edge_len) in neighs.iter().filter_map({
            let source = source.clone();
            move |neigh| {
                navmesh
                    .resolve_edge_data(source.clone(), neigh.clone())
                    .map(|(emeta, epi)| {
                        let edge_len = navmesh.access_edge_paths(epi).len();
                        (neigh, emeta.to_owned(), epi, edge_len)
                    })
            }
        }) {
            let source = source.clone();
            // A*-like remaining costs estimation
            let estimated_remaining =
                self.estimate_costs_for_source::<B::GapComment>(navmesh, neigh);
            for i in 0..=edge_len {
                let mut edge_paths = Box::from(navmesh.edge_paths);
                let mut navmesh = NavmeshRefMut {
                    nodes: navmesh.nodes,
                    edges: navmesh.edges,
                    edge_paths: &mut edge_paths,
                };
                navmesh
                    .access_edge_paths_mut(epi)
                    .with_borrow_mut(|mut j| j.insert(i, RelaxedPath::Normal(self.label.clone())));
                if let Some(new_task) = (*evaluate_navmesh)(
                    navmesh.as_ref(),
                    context,
                    InsertionInfo {
                        prev_node: source.clone(),
                        cur_node: neigh.clone(),
                        edge_meta: emeta.clone(),
                        epi,
                        intro: i,
                        maybe_new_goal: Some(self.label.clone()),
                    },
                )
                .map(|(costs, context)| Task {
                    goal_idx,
                    costs,
                    estimated_remaining,
                    estimated_remaining_goals,
                    edge_paths,
                    selected_node: neigh.clone(),
                    prev_node: source.clone(),
                    cur_intro: edge_len - i,
                    context,
                }) {
                    ret.push(new_task);
                }
            }
        }

        ret
    }
}

impl<B: NavmeshBase, Ctx> Task<B, Ctx>
where
    B::EtchedPath: PartialOrd,
    B::GapComment: Clone + PartialOrd,
    B::Scalar: num_traits::Float + num_traits::float::TotalOrder,
{
    pub fn run<F: EvaluateNavmesh<B, Ctx>>(
        self,
        env: &mut PmgAstar<B, Ctx>,
        evaluate_navmesh: F,
    ) -> ControlFlow<TaskResult<B, Ctx>, (Self, Vec<NavmeshIndex<B::PrimalNodeIndex>>)> {
        if let NavmeshIndex::Primal(primal) = &self.selected_node {
            if env.goals[self.goal_idx].target.contains(primal) {
                let Self {
                    goal_idx,
                    costs,
                    estimated_remaining: _,
                    estimated_remaining_goals: _,
                    edge_paths,
                    prev_node,
                    cur_intro,
                    selected_node: _,
                    context,
                } = self;
                return ControlFlow::Break(TaskResult {
                    goal_idx,
                    costs,
                    edge_paths,
                    prev_node,
                    cur_intro,
                    context,
                });
            } else {
                panic!("wrong primal node selected");
            }
        }
        let forks = self.progress(env, evaluate_navmesh);
        ControlFlow::Continue((self, forks))
    }

    /// progress to the next step, splitting the task into new tasks (make sure to call `done` beforehand)
    fn progress<F: EvaluateNavmesh<B, Ctx>>(
        &self,
        env: &mut PmgAstar<B, Ctx>,
        mut evaluate_navmesh: F,
    ) -> Vec<NavmeshIndex<B::PrimalNodeIndex>> {
        let goal_idx = self.goal_idx;
        let navmesh = NavmeshRef {
            nodes: &env.nodes,
            edges: &env.edges,
            edge_paths: &self.edge_paths,
        };
        let goal = &env.goals[goal_idx];
        let Some((_, other_ends)) = navmesh.planarr_find_all_other_ends(
            &self.selected_node,
            &self.prev_node,
            self.cur_intro,
            true,
        ) else {
            return Vec::new();
        };

        let mut ret = Vec::new();

        env.queue
            .extend(other_ends.filter_map(|(neigh, stop_data)| {
                if let NavmeshIndex::Primal(primal) = &neigh {
                    if !goal.target.contains(primal) {
                        return None;
                    }
                }
                // A*-like remaining costs estimation
                let estimated_remaining =
                    goal.estimate_costs_for_source::<B::GapComment>(navmesh, &neigh);
                let mut edge_paths = self.edge_paths.clone();
                let mut navmesh = NavmeshRefMut {
                    nodes: &env.nodes,
                    edges: &env.edges,
                    edge_paths: &mut edge_paths,
                };
                let (edge_meta, epi) = navmesh
                    .resolve_edge_data(self.selected_node.clone(), neigh.clone())
                    .unwrap();
                let edge_meta = edge_meta.to_owned();
                let cur_intro = navmesh.access_edge_paths_mut(epi).with_borrow_mut(|mut x| {
                    x.insert(
                        stop_data.insert_pos,
                        RelaxedPath::Normal(goal.label.clone()),
                    );
                    x.len() - stop_data.insert_pos - 1
                });
                ret.push(neigh.clone());
                evaluate_navmesh(
                    navmesh.as_ref(),
                    &self.context,
                    InsertionInfo {
                        prev_node: self.selected_node.clone(),
                        cur_node: neigh.clone(),
                        edge_meta,
                        epi,
                        intro: stop_data.insert_pos,
                        maybe_new_goal: None,
                    },
                )
                .map(|(costs, context)| Task {
                    goal_idx,
                    costs,
                    estimated_remaining,
                    estimated_remaining_goals: self.estimated_remaining_goals,
                    edge_paths,
                    selected_node: neigh.clone(),
                    prev_node: self.selected_node.clone(),
                    cur_intro,
                    context,
                })
            }));

        ret
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntermedResult<B: NavmeshBase, Ctx> {
    pub edge_paths: Box<[EdgePaths<B::EtchedPath, B::GapComment>]>,
    // TODO: maybe avoid these clones?
    pub context: Ctx,

    pub goal_idx: usize,
    pub forks: Vec<NavmeshIndex<B::PrimalNodeIndex>>,
    pub selected_node: NavmeshIndex<B::PrimalNodeIndex>,

    pub maybe_finished_goal: Option<B::Scalar>,
}

impl<B, Ctx> PmgAstar<B, Ctx>
where
    B: NavmeshBase,
    B::EtchedPath: PartialOrd,
    B::GapComment: Clone + PartialOrd,
    B::Scalar: Default
        + core::fmt::Debug
        + core::iter::Sum
        + num_traits::Float
        + num_traits::float::TotalOrder,
{
    /// * `evaluate_navmesh` calculates the exact cost of a given navmesh (lower cost is better)
    pub fn new<F: EvaluateNavmesh<B, Ctx>>(
        navmesh: &Navmesh<B>,
        goals: Vec<Goal<B::PrimalNodeIndex, B::EtchedPath>>,
        context: &Ctx,
        mut evaluate_navmesh: F,
    ) -> Self {
        let mut this = Self {
            queue: BinaryHeap::new(),
            goals: goals
                .into_iter()
                .map({
                    let navmesh = navmesh.as_ref();
                    move |i| i.prepare(navmesh)
                })
                .collect(),
            nodes: navmesh.nodes.clone(),
            edges: navmesh.edges.clone(),
        };

        // fill queue with first goal
        if let Some(first_goal) = this.goals.first() {
            this.queue = {
                let navmesh = NavmeshRef {
                    nodes: &this.nodes,
                    edges: &this.edges,
                    edge_paths: &navmesh.edge_paths,
                };
                first_goal.start_pmga(navmesh, 0, &this, context, &mut evaluate_navmesh)
            };
        }

        this
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    /// run one step of the path-search
    pub fn step<F: EvaluateNavmesh<B, Ctx>>(
        &mut self,
        mut evaluate_navmesh: F,
    ) -> ControlFlow<
        Option<(
            B::Scalar,
            Box<[EdgePaths<B::EtchedPath, B::GapComment>]>,
            Ctx,
        )>,
        IntermedResult<B, Ctx>,
    >
    where
        B::PrimalNodeIndex: core::fmt::Debug,
    {
        let Some(task) = self.queue.pop() else {
            log::info!("found no complete result");
            return ControlFlow::Break(None);
        };
        ControlFlow::Continue(match task.run(self, &mut evaluate_navmesh) {
            ControlFlow::Break(taskres) => {
                let next_goal_idx = taskres.goal_idx + 1;
                let navmesh = NavmeshRef {
                    nodes: &self.nodes,
                    edges: &self.edges,
                    edge_paths: &taskres.edge_paths,
                };
                let edge_count = taskres.edge_paths.iter().map(|i| i.len()).sum::<usize>();
                match self.goals.get(next_goal_idx) {
                    None => {
                        // done with all goals
                        log::info!(
                            "found result with {} edges and costs {:?}",
                            edge_count,
                            taskres.costs
                        );
                        return ControlFlow::Break(Some((
                            taskres.costs,
                            taskres.edge_paths,
                            taskres.context,
                        )));
                    }
                    Some(next_goal) => {
                        // prepare next goal
                        log::debug!(
                            "found partial result (goal {}) with {} edges and costs {:?}",
                            taskres.goal_idx,
                            edge_count,
                            taskres.costs,
                        );
                        let mut tmp = next_goal.start_pmga(
                            navmesh,
                            next_goal_idx,
                            self,
                            &taskres.context,
                            &mut evaluate_navmesh,
                        );
                        let forks = tmp.iter().map(|i| i.selected_node.clone()).collect();
                        self.queue.append(&mut tmp);
                        IntermedResult {
                            goal_idx: taskres.goal_idx,
                            forks,
                            edge_paths: taskres.edge_paths,
                            context: taskres.context,
                            selected_node: NavmeshIndex::Primal(next_goal.source.clone()),
                            maybe_finished_goal: Some(taskres.costs),
                        }
                    }
                }
            }
            ControlFlow::Continue((task, forks)) => {
                // task got further branched
                IntermedResult {
                    goal_idx: task.goal_idx,
                    forks,
                    edge_paths: task.edge_paths,
                    context: task.context,
                    selected_node: task.selected_node,
                    maybe_finished_goal: None,
                }
            }
        })
    }
}
