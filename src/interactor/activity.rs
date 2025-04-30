// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::ops::ControlFlow;

use enum_dispatch::enum_dispatch;
use geo::Point;
use thiserror::Error;

use crate::{
    autorouter::{
        execution::ExecutionStepper,
        invoker::{
            GetGhosts, GetMaybeNavcord, GetMaybeNavmesh, GetNavmeshDebugTexts, GetObstacles,
            Invoker, InvokerError,
        },
    },
    board::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    interactor::interaction::{InteractionError, InteractionStepper},
    router::{
        navcord::NavcordStepper,
        navmesh::{Navmesh, NavvertexIndex},
    },
    stepper::{Abort, Step},
};

/// Stores the interactive input data from the user
pub struct InteractiveInput {
    pub pointer_pos: Point,
    pub dt: f32,
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
    GetMaybeNavmesh,
    GetMaybeNavcord,
    GetGhosts,
    GetObstacles,
    GetNavmeshDebugTexts
)]
pub enum ActivityStepper {
    Interaction(InteractionStepper),
    Execution(ExecutionStepper),
}

impl<M: AccessMesadata> Step<ActivityContext<'_, M>, String> for ActivityStepper {
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

impl<M: AccessMesadata> Abort<ActivityContext<'_, M>> for ActivityStepper {
    fn abort(&mut self, context: &mut ActivityContext<M>) {
        match self {
            ActivityStepper::Interaction(interaction) => interaction.abort(context),
            ActivityStepper::Execution(execution) => {
                execution.finish(context.invoker);
            } // TODO.
        };
    }
}

/// An ActivityStepper that preserves its status
pub struct ActivityStepperWithStatus {
    activity: ActivityStepper,
    maybe_status: Option<ControlFlow<String>>,
}

impl ActivityStepperWithStatus {
    pub fn new_execution(execution: ExecutionStepper) -> ActivityStepperWithStatus {
        Self {
            activity: ActivityStepper::Execution(execution),
            maybe_status: None,
        }
    }

    pub fn maybe_status(&self) -> Option<ControlFlow<String>> {
        self.maybe_status.clone()
    }
}

impl<M: AccessMesadata> Step<ActivityContext<'_, M>, String> for ActivityStepperWithStatus {
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

impl<M: AccessMesadata> Abort<ActivityContext<'_, M>> for ActivityStepperWithStatus {
    fn abort(&mut self, context: &mut ActivityContext<M>) {
        self.maybe_status = Some(ControlFlow::Break(String::from("aborted")));
        self.activity.abort(context);
    }
}

impl GetMaybeNavmesh for ActivityStepperWithStatus {
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        self.activity.maybe_navmesh()
    }
}

impl GetMaybeNavcord for ActivityStepperWithStatus {
    fn maybe_navcord(&self) -> Option<&NavcordStepper> {
        self.activity.maybe_navcord()
    }
}

impl GetGhosts for ActivityStepperWithStatus {
    fn ghosts(&self) -> &[PrimitiveShape] {
        self.activity.ghosts()
    }
}

impl GetObstacles for ActivityStepperWithStatus {
    fn obstacles(&self) -> &[PrimitiveIndex] {
        self.activity.obstacles()
    }
}

impl GetNavmeshDebugTexts for ActivityStepperWithStatus {
    fn navvertex_debug_text(&self, navvertex: NavvertexIndex) -> Option<&str> {
        self.activity.navvertex_debug_text(navvertex)
    }

    fn navedge_debug_text(&self, navedge: (NavvertexIndex, NavvertexIndex)) -> Option<&str> {
        self.activity.navedge_debug_text(navedge)
    }
}
