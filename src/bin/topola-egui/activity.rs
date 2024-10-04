use thiserror::Error;
use topola::{
    autorouter::{
        execution::ExecutionStepper,
        invoker::{
            GetGhosts, GetMaybeNavmesh, GetMaybeTrace, GetObstacles, Invoker, InvokerError,
            InvokerStatus,
        },
    },
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    router::{navmesh::Navmesh, trace::TraceStepper},
    specctra::mesadata::SpecctraMesadata,
    stepper::{Abort, Step},
};

pub struct ActivityContext<'a> {
    pub invoker: &'a mut Invoker<SpecctraMesadata>,
}

#[derive(Debug, Clone)]
pub enum ActivityStatus {
    Running,
    Finished(String),
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
    Invoker(#[from] InvokerError),
}

pub enum ActivityStepper {
    // There will be another variant for interactive activities here soon. (TODO)
    Execution(ExecutionStepper),
}

impl Step<ActivityContext<'_>, ActivityStatus, ActivityError, ()> for ActivityStepper {
    fn step(&mut self, context: &mut ActivityContext) -> Result<ActivityStatus, ActivityError> {
        match self {
            ActivityStepper::Execution(execution) => Ok(execution.step(context.invoker)?.into()),
        }
    }
}

impl Abort<ActivityContext<'_>> for ActivityStepper {
    fn abort(&mut self, context: &mut ActivityContext) {
        match self {
            ActivityStepper::Execution(execution) => execution.finish(context.invoker), // TODO.
        };
    }
}

impl GetMaybeNavmesh for ActivityStepper {
    /// Implemented manually instead of with `enum_dispatch` because it doesn't work across crates.
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        match self {
            ActivityStepper::Execution(execution) => execution.maybe_navmesh(),
        }
    }
}

impl GetMaybeTrace for ActivityStepper {
    /// Implemented manually instead of with `enum_dispatch` because it doesn't work across crates.
    fn maybe_trace(&self) -> Option<&TraceStepper> {
        match self {
            ActivityStepper::Execution(execution) => execution.maybe_trace(),
        }
    }
}

impl GetGhosts for ActivityStepper {
    /// Implemented manually instead of with `enum_dispatch` because it doesn't work across crates.
    fn ghosts(&self) -> &[PrimitiveShape] {
        match self {
            ActivityStepper::Execution(execution) => execution.ghosts(),
        }
    }
}

impl GetObstacles for ActivityStepper {
    /// Implemented manually instead of with `enum_dispatch` because it doesn't work across crates.
    fn obstacles(&self) -> &[PrimitiveIndex] {
        match self {
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
