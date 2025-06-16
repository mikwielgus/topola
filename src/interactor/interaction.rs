// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use core::ops::ControlFlow;
use thiserror::Error;

use crate::{
    autorouter::invoker::{
        GetGhosts, GetMaybeNavcord, GetMaybeThetastarStepper, GetNavmeshDebugTexts, GetObstacles,
        Invoker,
    },
    board::AccessMesadata,
    stepper::{Abort, OnEvent, Step},
};

use super::{
    activity::{ActivityContext, InteractiveEvent},
    route_plan::RoutePlan,
};

#[derive(Error, Debug, Clone)]
pub enum InteractionError {
    #[error("nothing to interact with")]
    NothingToInteract,
}

pub enum InteractionStepper {
    // Examples of interactions:
    // - interactively routing a track
    // - interactively moving a footprint.
    RoutePlan(RoutePlan),
}

impl<M: AccessMesadata> Step<ActivityContext<'_, M>, String> for InteractionStepper {
    type Error = InteractionError;

    fn step(
        &mut self,
        context: &mut ActivityContext<M>,
    ) -> Result<ControlFlow<String>, InteractionError> {
        match self {
            Self::RoutePlan(rp) => rp.step(context),
        }
    }
}

impl<M: AccessMesadata> Abort<Invoker<M>> for InteractionStepper {
    fn abort(&mut self, context: &mut Invoker<M>) {
        match self {
            Self::RoutePlan(rp) => rp.abort(context),
        }
    }
}

impl<M: AccessMesadata> OnEvent<ActivityContext<'_, M>, InteractiveEvent> for InteractionStepper {
    type Output = Result<(), InteractionError>;

    fn on_event(
        &mut self,
        context: &mut ActivityContext<M>,
        event: InteractiveEvent,
    ) -> Result<(), InteractionError> {
        match self {
            Self::RoutePlan(rp) => rp.on_event(context, event),
        }
    }
}

impl GetGhosts for InteractionStepper {}
impl GetMaybeThetastarStepper for InteractionStepper {}
impl GetMaybeNavcord for InteractionStepper {}
impl GetNavmeshDebugTexts for InteractionStepper {}
impl GetObstacles for InteractionStepper {}
