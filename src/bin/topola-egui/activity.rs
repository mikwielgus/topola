use thiserror::Error;
use topola::{
    autorouter::{
        execution::ExecutionStepper,
        invoker::{
            GetGhosts, GetMaybeNavmesh, GetMaybeTrace, GetObstacles, Invoker, InvokerError,
            InvokerStatus,
        },
    },
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    router::{navmesh::Navmesh, trace::TraceStepper},
    specctra::mesadata::SpecctraMesadata,
    stepper::{Abort, Step},
};

use crate::interaction::{
    InteractionContext, InteractionError, InteractionStatus, InteractionStepper,
};

pub struct ActivityContext<'a> {
    pub interaction: InteractionContext,
    pub invoker: &'a mut Invoker<SpecctraMesadata>,
}

#[derive(Debug, Clone)]
pub enum ActivityStatus {
    Running,
    Finished(String),
}

impl From<InteractionStatus> for ActivityStatus {
    fn from(status: InteractionStatus) -> Self {
        match status {
            InteractionStatus::Running => ActivityStatus::Running,
            InteractionStatus::Finished(msg) => ActivityStatus::Finished(msg),
        }
    }
}

impl From<InvokerStatus> for ActivityStatus {
    fn from(status: InvokerStatus) -> Self {
        match status {
            InvokerStatus::Running => ActivityStatus::Running,
            InvokerStatus::Finished(msg) => ActivityStatus::Finished(msg),
        }
    }
}

impl TryInto<()> for ActivityStatus {
    type Error = ();
    fn try_into(self) -> Result<(), ()> {
        match self {
            ActivityStatus::Running => Err(()),
            ActivityStatus::Finished(..) => Ok(()),
        }
    }
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

impl Step<ActivityContext<'_>, ActivityStatus, ActivityError, ()> for ActivityStepper {
    fn step(&mut self, context: &mut ActivityContext) -> Result<ActivityStatus, ActivityError> {
        match self {
            ActivityStepper::Interaction(interaction) => {
                Ok(interaction.step(&mut context.interaction)?.into())
            }
            ActivityStepper::Execution(execution) => Ok(execution.step(context.invoker)?.into()),
        }
    }
}

impl Abort<ActivityContext<'_>> for ActivityStepper {
    fn abort(&mut self, context: &mut ActivityContext) {
        match self {
            ActivityStepper::Interaction(interaction) => {
                Ok(interaction.abort(&mut context.interaction))
            }
            ActivityStepper::Execution(execution) => execution.finish(context.invoker), // TODO.
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

impl GetMaybeTrace for ActivityStepper {
    /// Implemented manually instead of with `enum_dispatch` because it doesn't work across crates.
    fn maybe_trace(&self) -> Option<&TraceStepper> {
        match self {
            ActivityStepper::Interaction(interaction) => interaction.maybe_trace(),
            ActivityStepper::Execution(execution) => execution.maybe_trace(),
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
    maybe_status: Option<ActivityStatus>,
}

impl ActivityStepperWithStatus {
    pub fn new_execution(execution: ExecutionStepper) -> ActivityStepperWithStatus {
        Self {
            activity: ActivityStepper::Execution(execution),
            maybe_status: None,
        }
    }

    pub fn maybe_status(&self) -> Option<ActivityStatus> {
        self.maybe_status.clone()
    }
}

impl Step<ActivityContext<'_>, ActivityStatus, ActivityError, ()> for ActivityStepperWithStatus {
    fn step(&mut self, context: &mut ActivityContext) -> Result<ActivityStatus, ActivityError> {
        let status = self.activity.step(context)?;
        self.maybe_status = Some(status.clone());
        Ok(status.into())
    }
}

impl Abort<ActivityContext<'_>> for ActivityStepperWithStatus {
    fn abort(&mut self, context: &mut ActivityContext) {
        self.maybe_status = Some(ActivityStatus::Finished(String::from("aborted")));
        self.activity.abort(context);
    }
}

impl GetMaybeNavmesh for ActivityStepperWithStatus {
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        self.activity.maybe_navmesh()
    }
}

impl GetMaybeTrace for ActivityStepperWithStatus {
    fn maybe_trace(&self) -> Option<&TraceStepper> {
        self.activity.maybe_trace()
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
