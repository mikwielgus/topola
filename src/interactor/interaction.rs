use std::ops::ControlFlow;

use thiserror::Error;

use crate::{
    autorouter::invoker::{GetGhosts, GetMaybeNavcord, GetMaybeNavmesh, GetObstacles},
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    router::{navcord::NavcordStepper, navmesh::Navmesh},
    stepper::{Abort, Step},
};

use super::activity::ActivityContext;

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

impl<'a, M: AccessMesadata> Step<ActivityContext<'a, M>, String> for InteractionStepper {
    type Error = InteractionError;

    fn step(
        &mut self,
        context: &mut ActivityContext<M>,
    ) -> Result<ControlFlow<String>, InteractionError> {
        Ok(ControlFlow::Break(String::from("")))
    }
}

impl<'a, M: AccessMesadata> Abort<ActivityContext<'a, M>> for InteractionStepper {
    fn abort(&mut self, context: &mut ActivityContext<M>) {
        todo!();
    }
}

impl GetMaybeNavmesh for InteractionStepper {
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        todo!()
    }
}

impl GetMaybeNavcord for InteractionStepper {
    fn maybe_navcord(&self) -> Option<&NavcordStepper> {
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
