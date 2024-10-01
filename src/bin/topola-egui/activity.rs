use thiserror::Error;
use topola::{
    autorouter::{
        command::CommandStepper,
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
    step::Step,
};

#[derive(Error, Debug, Clone)]
pub enum ActivityError {
    #[error(transparent)]
    Invoker(#[from] InvokerError),
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

pub enum ActivityStepper {
    // There will be another variant for interactive activities here soon. (TODO)
    Command(CommandStepper),
}

impl Step<Invoker<SpecctraMesadata>, ActivityStatus, ActivityError, ()> for ActivityStepper {
    fn step(
        &mut self,
        invoker: &mut Invoker<SpecctraMesadata>,
    ) -> Result<ActivityStatus, ActivityError> {
        match self {
            ActivityStepper::Command(execute) => Ok(execute.step(invoker)?.into()),
        }
    }
}

impl GetMaybeNavmesh for ActivityStepper {
    /// Implemented manually instead of with `enum_dispatch` because it doesn't work across crates.
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        match self {
            ActivityStepper::Command(execute) => execute.maybe_navmesh(),
        }
    }
}

impl GetMaybeTrace for ActivityStepper {
    /// Implemented manually instead of with `enum_dispatch` because it doesn't work across crates.
    fn maybe_trace(&self) -> Option<&TraceStepper> {
        match self {
            ActivityStepper::Command(execute) => execute.maybe_trace(),
        }
    }
}

impl GetGhosts for ActivityStepper {
    /// Implemented manually instead of with `enum_dispatch` because it doesn't work across crates.
    fn ghosts(&self) -> &[PrimitiveShape] {
        match self {
            ActivityStepper::Command(execute) => execute.ghosts(),
        }
    }
}

impl GetObstacles for ActivityStepper {
    /// Implemented manually instead of with `enum_dispatch` because it doesn't work across crates.
    fn obstacles(&self) -> &[PrimitiveIndex] {
        match self {
            ActivityStepper::Command(execute) => execute.obstacles(),
        }
    }
}

pub struct ActivityWithStatus {
    activity: ActivityStepper,
    maybe_status: Option<ActivityStatus>,
}

impl ActivityWithStatus {
    pub fn new_execute(execute: CommandStepper) -> ActivityWithStatus {
        Self {
            activity: ActivityStepper::Command(execute),
            maybe_status: None,
        }
    }

    pub fn step(
        &mut self,
        invoker: &mut Invoker<SpecctraMesadata>,
    ) -> Result<ActivityStatus, ActivityError> {
        let status = self.activity.step(invoker)?;
        self.maybe_status = Some(status.clone());
        Ok(status.into())
    }

    pub fn maybe_status(&self) -> Option<ActivityStatus> {
        self.maybe_status.clone()
    }
}

impl GetMaybeNavmesh for ActivityWithStatus {
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        self.activity.maybe_navmesh()
    }
}

impl GetMaybeTrace for ActivityWithStatus {
    fn maybe_trace(&self) -> Option<&TraceStepper> {
        self.activity.maybe_trace()
    }
}

impl GetGhosts for ActivityWithStatus {
    fn ghosts(&self) -> &[PrimitiveShape] {
        self.activity.ghosts()
    }
}

impl GetObstacles for ActivityWithStatus {
    fn obstacles(&self) -> &[PrimitiveIndex] {
        self.activity.obstacles()
    }
}
