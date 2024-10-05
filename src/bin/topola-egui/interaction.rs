use thiserror::Error;
use topola::{
    autorouter::invoker::{GetGhosts, GetMaybeNavmesh, GetMaybeTrace, GetObstacles},
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    router::{navmesh::Navmesh, trace::TraceStepper},
    stepper::{Abort, Step},
};

use crate::activity::ActivityStepperWithStatus;

pub struct InteractionContext {
    // Empty for now.
    // For example, this will contain mouse pointer position.
    // (we will need an additional struct to hold a reference to a `Board<...>`)
}

#[derive(Debug, Clone)]
pub enum InteractionStatus {
    Running,
    Finished(String),
}

impl TryInto<()> for InteractionStatus {
    type Error = ();
    fn try_into(self) -> Result<(), ()> {
        match self {
            InteractionStatus::Running => Err(()),
            InteractionStatus::Finished(..) => Ok(()),
        }
    }
}

#[derive(Error, Debug, Clone)]
pub enum InteractionError {
    #[error("nothing to interact with")]
    NothingToInteract,
}

pub enum InteractionStepper {
    // No interactions yet. This is only an empty skeleton for now.
    // Examples of interactions:
    // - interactively routing a track
    // - interactively moving a footprint.
}

impl Step<InteractionContext, InteractionStatus, InteractionError, ()> for InteractionStepper {
    fn step(
        &mut self,
        invoker: &mut InteractionContext,
    ) -> Result<InteractionStatus, InteractionError> {
        Ok(InteractionStatus::Finished(String::from("")))
    }
}

impl Abort<InteractionContext> for InteractionStepper {
    fn abort(&mut self, context: &mut InteractionContext) {
        todo!();
    }
}

impl GetMaybeNavmesh for InteractionStepper {
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        todo!()
    }
}

impl GetMaybeTrace for InteractionStepper {
    fn maybe_trace(&self) -> Option<&TraceStepper> {
        todo!()
    }
}

impl GetGhosts for InteractionStepper {
    fn ghosts(&self) -> &[PrimitiveShape] {
        todo!()
    }
}

impl GetObstacles for InteractionStepper {
    fn obstacles(&self) -> &[PrimitiveIndex] {
        todo!()
    }
}
