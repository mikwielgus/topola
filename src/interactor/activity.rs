use std::ops::ControlFlow;

use thiserror::Error;

use crate::{
    autorouter::{
        execution::ExecutionStepper,
        invoker::{
            GetGhosts, GetMaybeNavcord, GetMaybeNavmesh, GetObstacles, Invoker, InvokerError,
        },
    },
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    interactor::interaction::{InteractionContext, InteractionError, InteractionStepper},
    router::{navcord::NavcordStepper, navmesh::Navmesh},
    stepper::{Abort, Step},
};

pub struct ActivityContext<'a, M: AccessMesadata> {
    pub interaction: InteractionContext,
    pub invoker: &'a mut Invoker<M>,
}

#[derive(Error, Debug, Clone)]
pub enum ActivityError {
    #[error(transparent)]
    Interaction(#[from] InteractionError),
    #[error(transparent)]
    Invoker(#[from] InvokerError),
}

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
            ActivityStepper::Interaction(interaction) => {
                Ok(interaction.step(&mut context.interaction)?)
            }
            ActivityStepper::Execution(execution) => Ok(execution.step(context.invoker)?),
        }
    }
}

impl<M: AccessMesadata> Abort<ActivityContext<'_, M>> for ActivityStepper {
    fn abort(&mut self, context: &mut ActivityContext<M>) {
        match self {
            ActivityStepper::Interaction(interaction) => {
                interaction.abort(&mut context.interaction)
            }
            ActivityStepper::Execution(execution) => {
                execution.finish(context.invoker);
            } // TODO.
        };
    }
}

impl GetMaybeNavmesh for ActivityStepper {
    /// Implemented manually instead of with `enum_dispatch` because it doesn't work across crates.
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        match self {
            ActivityStepper::Interaction(interaction) => interaction.maybe_navmesh(),
            ActivityStepper::Execution(execution) => execution.maybe_navmesh(),
        }
    }
}

impl GetMaybeNavcord for ActivityStepper {
    /// Implemented manually instead of with `enum_dispatch` because it doesn't work across crates.
    fn maybe_navcord(&self) -> Option<&NavcordStepper> {
        match self {
            ActivityStepper::Interaction(interaction) => interaction.maybe_navcord(),
            ActivityStepper::Execution(execution) => execution.maybe_navcord(),
        }
    }
}

impl GetGhosts for ActivityStepper {
    /// Implemented manually instead of with `enum_dispatch` because it doesn't work across crates.
    fn ghosts(&self) -> &[PrimitiveShape] {
        match self {
            ActivityStepper::Interaction(interaction) => interaction.ghosts(),
            ActivityStepper::Execution(execution) => execution.ghosts(),
        }
    }
}

impl GetObstacles for ActivityStepper {
    /// Implemented manually instead of with `enum_dispatch` because it doesn't work across crates.
    fn obstacles(&self) -> &[PrimitiveIndex] {
        match self {
            ActivityStepper::Interaction(interaction) => interaction.obstacles(),
            ActivityStepper::Execution(execution) => execution.obstacles(),
        }
    }
}

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
        Ok(status.into())
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
