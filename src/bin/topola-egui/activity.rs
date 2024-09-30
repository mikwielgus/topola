use thiserror::Error;
use topola::{
    autorouter::{
        execute::Execute,
        invoker::{
            GetGhosts, GetMaybeNavmesh, GetMaybeTrace, GetObstacles, Invoker, InvokerError,
            InvokerStatus,
        },
    },
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    router::{navmesh::Navmesh, trace::Trace},
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

pub enum Activity {
    // There will be another variant for interactive activities here soon. (TODO)
    Execute(Execute),
}

impl Step<Invoker<SpecctraMesadata>, ActivityStatus, ActivityError, ()> for Activity {
    fn step(
        &mut self,
        invoker: &mut Invoker<SpecctraMesadata>,
    ) -> Result<ActivityStatus, ActivityError> {
        match self {
            Activity::Execute(execute) => Ok(execute.step(invoker)?.into()),
        }
    }
}

impl GetMaybeNavmesh for Activity {
    /// Implemented manually instead of with `enum_dispatch` because it doesn't work across crates.
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        match self {
            Activity::Execute(execute) => execute.maybe_navmesh(),
        }
    }
}

impl GetMaybeTrace for Activity {
    /// Implemented manually instead of with `enum_dispatch` because it doesn't work across crates.
    fn maybe_trace(&self) -> Option<&Trace> {
        match self {
            Activity::Execute(execute) => execute.maybe_trace(),
        }
    }
}

impl GetGhosts for Activity {
    /// Implemented manually instead of with `enum_dispatch` because it doesn't work across crates.
    fn ghosts(&self) -> &[PrimitiveShape] {
        match self {
            Activity::Execute(execute) => execute.ghosts(),
        }
    }
}

impl GetObstacles for Activity {
    /// Implemented manually instead of with `enum_dispatch` because it doesn't work across crates.
    fn obstacles(&self) -> &[PrimitiveIndex] {
        match self {
            Activity::Execute(execute) => execute.obstacles(),
        }
    }
}

pub struct ActivityWithStatus {
    activity: Activity,
    maybe_status: Option<ActivityStatus>,
}

impl ActivityWithStatus {
    pub fn new_execute(execute: Execute) -> ActivityWithStatus {
        Self {
            activity: Activity::Execute(execute),
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
    fn maybe_trace(&self) -> Option<&Trace> {
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
