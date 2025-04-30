// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::ops::ControlFlow;

use thiserror::Error;

use crate::{
    autorouter::invoker::{
        GetGhosts, GetMaybeNavcord, GetMaybeNavmesh, GetNavmeshDebugTexts, GetObstacles,
    },
    board::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    router::{
        navcord::NavcordStepper,
        navmesh::{Navmesh, NavvertexIndex},
    },
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

impl<M: AccessMesadata> Step<ActivityContext<'_, M>, String> for InteractionStepper {
    type Error = InteractionError;

    fn step(
        &mut self,
        _context: &mut ActivityContext<M>,
    ) -> Result<ControlFlow<String>, InteractionError> {
        Ok(ControlFlow::Break(String::from("")))
    }
}

impl<M: AccessMesadata> Abort<ActivityContext<'_, M>> for InteractionStepper {
    fn abort(&mut self, _context: &mut ActivityContext<M>) {
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

impl GetNavmeshDebugTexts for InteractionStepper {
    fn navvertex_debug_text(&self, navvertex: NavvertexIndex) -> Option<&str> {
        todo!()
    }

    fn navedge_debug_text(&self, navedge: (NavvertexIndex, NavvertexIndex)) -> Option<&str> {
        todo!()
    }
}
