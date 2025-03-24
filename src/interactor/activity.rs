// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use core::ops::ControlFlow;

use enum_dispatch::enum_dispatch;
use geo::geometry::{LineString, Point};
use thiserror::Error;

use crate::{
    autorouter::{
        execution::ExecutionStepper,
        invoker::{
            GetActivePolygons, GetGhosts, GetMaybeNavcord, GetMaybeThetastarStepper,
            GetMaybeTopoNavmesh, GetNavmeshDebugTexts, GetObstacles, GetPolygonalBlockers, Invoker,
            InvokerError,
        },
    },
    board::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    graph::GenericIndex,
    interactor::interaction::{InteractionError, InteractionStepper},
    layout::poly::PolyWeight,
    router::{
        navcord::Navcord,
        navmesh::{Navmesh, NavnodeIndex},
        ng,
        thetastar::ThetastarStepper,
    },
    stepper::{Abort, OnEvent, Step},
};

/// Stores the interactive input data from the user
pub struct InteractiveInput {
    pub active_layer: Option<usize>,
    pub pointer_pos: Point,
    pub dt: f32,
}

#[derive(Clone, Copy, Debug)]
pub enum InteractiveEventKind {
    PointerPrimaryButtonClicked,
    PointerPrimaryButtonDragStarted,
    PointerPrimaryButtonDragStopped,
    PointerSecondaryButtonClicked,
}

/// An event received from the user
pub struct InteractiveEvent {
    pub kind: InteractiveEventKind,

    /// `true` if the `Ctrl` key pressed during the event
    pub ctrl: bool,

    /// `true` if the `Shift` key pressed during the event
    pub shift: bool,
}

/// This is the execution context passed to the stepper on each step
pub struct ActivityContext<'a, M> {
    pub interactive_input: &'a InteractiveInput,
    pub invoker: &'a mut Invoker<M>,
}

#[derive(Error, Debug, Clone)]
pub enum ActivityError {
    #[error(transparent)]
    Interaction(#[from] InteractionError),
    #[error(transparent)]
    Invoker(#[from] InvokerError),
}

/// An activity is either an interaction or an execution
#[enum_dispatch(
    GetActivePolygons,
    GetGhosts,
    GetMaybeNavcord,
    GetMaybeThetastarStepper,
    GetMaybeTopoNavmesh,
    GetNavmeshDebugTexts,
    GetObstacles,
    GetPolygonalBlockers
)]
pub enum ActivityStepper<M> {
    Interaction(InteractionStepper),
    Execution(ExecutionStepper<M>),
}

impl<M: AccessMesadata + Clone> Step<ActivityContext<'_, M>, String> for ActivityStepper<M> {
    type Error = ActivityError;

    fn step(
        &mut self,
        context: &mut ActivityContext<M>,
    ) -> Result<ControlFlow<String>, ActivityError> {
        match self {
            ActivityStepper::Interaction(interaction) => Ok(interaction.step(context)?),
            ActivityStepper::Execution(execution) => Ok(execution.step(context.invoker)?),
        }
    }
}

impl<M: AccessMesadata + Clone> Abort<Invoker<M>> for ActivityStepper<M> {
    fn abort(&mut self, context: &mut Invoker<M>) {
        match self {
            ActivityStepper::Interaction(interaction) => interaction.abort(context),
            ActivityStepper::Execution(execution) => execution.abort(context),
        }
    }
}

impl<M: AccessMesadata> OnEvent<ActivityContext<'_, M>, InteractiveEvent> for ActivityStepper<M> {
    type Output = Result<(), InteractionError>;

    fn on_event(
        &mut self,
        context: &mut ActivityContext<M>,
        event: InteractiveEvent,
    ) -> Result<(), InteractionError> {
        match self {
            ActivityStepper::Interaction(interaction) => interaction.on_event(context, event),
            ActivityStepper::Execution(_) => Ok(()),
        }
    }
}

/// An ActivityStepper that preserves its status
pub struct ActivityStepperWithStatus<M> {
    activity: ActivityStepper<M>,
    maybe_status: Option<ControlFlow<String>>,
}

impl<M> ActivityStepperWithStatus<M> {
    pub fn new_execution(execution: ExecutionStepper<M>) -> Self {
        Self {
            activity: ActivityStepper::Execution(execution),
            maybe_status: None,
        }
    }

    pub fn new_interaction(interaction: InteractionStepper) -> Self {
        Self {
            activity: ActivityStepper::Interaction(interaction),
            maybe_status: None,
        }
    }

    pub fn activity(&self) -> &ActivityStepper<M> {
        &self.activity
    }

    pub fn maybe_status(&self) -> Option<ControlFlow<String>> {
        self.maybe_status.clone()
    }
}

impl<M: AccessMesadata + Clone> Step<ActivityContext<'_, M>, String>
    for ActivityStepperWithStatus<M>
{
    type Error = ActivityError;

    fn step(
        &mut self,
        context: &mut ActivityContext<M>,
    ) -> Result<ControlFlow<String>, ActivityError> {
        let status = self.activity.step(context)?;
        self.maybe_status = Some(status.clone());
        Ok(status)
    }
}

impl<M: AccessMesadata + Clone> Abort<Invoker<M>> for ActivityStepperWithStatus<M> {
    fn abort(&mut self, context: &mut Invoker<M>) {
        self.maybe_status = Some(ControlFlow::Break(String::from("aborted")));
        self.activity.abort(context);
    }
}

impl<M: AccessMesadata + Clone> OnEvent<ActivityContext<'_, M>, InteractiveEvent>
    for ActivityStepperWithStatus<M>
{
    type Output = Result<(), InteractionError>;

    fn on_event(
        &mut self,
        context: &mut ActivityContext<M>,
        event: InteractiveEvent,
    ) -> Result<(), InteractionError> {
        self.activity.on_event(context, event)
    }
}

impl<M> GetActivePolygons for ActivityStepperWithStatus<M> {
    fn active_polygons(&self) -> &[GenericIndex<PolyWeight>] {
        self.activity.active_polygons()
    }
}

impl<M> GetMaybeThetastarStepper for ActivityStepperWithStatus<M> {
    fn maybe_thetastar(&self) -> Option<&ThetastarStepper<Navmesh, f64>> {
        self.activity.maybe_thetastar()
    }
}

impl<M> GetMaybeTopoNavmesh for ActivityStepperWithStatus<M> {
    fn maybe_topo_navmesh(&self) -> Option<ng::pie::navmesh::NavmeshRef<'_, ng::PieNavmeshBase>> {
        self.activity.maybe_topo_navmesh()
    }
}

impl<M> GetMaybeNavcord for ActivityStepperWithStatus<M> {
    fn maybe_navcord(&self) -> Option<&Navcord> {
        self.activity.maybe_navcord()
    }
}

impl<M> GetGhosts for ActivityStepperWithStatus<M> {
    fn ghosts(&self) -> &[PrimitiveShape] {
        self.activity.ghosts()
    }
}

impl<M> GetPolygonalBlockers for ActivityStepperWithStatus<M> {
    fn polygonal_blockers(&self) -> &[LineString] {
        self.activity.polygonal_blockers()
    }
}

impl<M> GetObstacles for ActivityStepperWithStatus<M> {
    fn obstacles(&self) -> &[PrimitiveIndex] {
        self.activity.obstacles()
    }
}

impl<M> GetNavmeshDebugTexts for ActivityStepperWithStatus<M> {
    fn navnode_debug_text(&self, navnode: NavnodeIndex) -> Option<&str> {
        self.activity.navnode_debug_text(navnode)
    }

    fn navedge_debug_text(&self, navedge: (NavnodeIndex, NavnodeIndex)) -> Option<&str> {
        self.activity.navedge_debug_text(navedge)
    }
}
