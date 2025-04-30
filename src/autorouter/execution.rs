// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::ops::ControlFlow;

use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use crate::{
    board::AccessMesadata,
    layout::{via::ViaWeight, LayoutEdit},
    stepper::Step,
};

use super::{
    autoroute::AutorouteExecutionStepper,
    compare_detours::CompareDetoursExecutionStepper,
    invoker::{Invoker, InvokerError},
    measure_length::MeasureLengthExecutionStepper,
    place_via::PlaceViaExecutionStepper,
    remove_bands::RemoveBandsExecutionStepper,
    selection::{BandSelection, PinSelection},
    Autorouter, AutorouterOptions,
};

type Type = PinSelection;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    Autoroute(PinSelection, AutorouterOptions),
    PlaceVia(ViaWeight),
    RemoveBands(BandSelection),
    CompareDetours(Type, AutorouterOptions),
    MeasureLength(BandSelection),
}

#[enum_dispatch(
    GetMaybeNavmesh,
    GetMaybeNavcord,
    GetGhosts,
    GetObstacles,
    GetNavmeshDebugTexts
)]
pub enum ExecutionStepper {
    Autoroute(AutorouteExecutionStepper),
    PlaceVia(PlaceViaExecutionStepper),
    RemoveBands(RemoveBandsExecutionStepper),
    CompareDetours(CompareDetoursExecutionStepper),
    MeasureLength(MeasureLengthExecutionStepper),
}

impl ExecutionStepper {
    fn step_catch_err<M: AccessMesadata>(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<ControlFlow<(Option<LayoutEdit>, String)>, InvokerError> {
        Ok(match self {
            ExecutionStepper::Autoroute(autoroute) => match autoroute.step(autorouter)? {
                ControlFlow::Continue(..) => ControlFlow::Continue(()),
                ControlFlow::Break(edit) => {
                    ControlFlow::Break((edit, "finished autorouting".to_string()))
                }
            },
            ExecutionStepper::PlaceVia(place_via) => {
                let edit = place_via.doit(autorouter)?;
                ControlFlow::Break((edit, "finished placing via".to_string()))
            }
            ExecutionStepper::RemoveBands(remove_bands) => {
                let edit = remove_bands.doit(autorouter)?;
                ControlFlow::Break((edit, "finished removing bands".to_string()))
            }
            ExecutionStepper::CompareDetours(compare_detours) => {
                match compare_detours.step(autorouter)? {
                    ControlFlow::Continue(()) => ControlFlow::Continue(()),
                    ControlFlow::Break((total_length1, total_length2)) => ControlFlow::Break((
                        None,
                        format!(
                            "total detour lengths are {} and {}",
                            total_length1, total_length2
                        ),
                    )),
                }
            }
            ExecutionStepper::MeasureLength(measure_length) => {
                let length = measure_length.doit(autorouter)?;
                ControlFlow::Break((None, format!("Total length of selected bands: {}", length)))
            }
        })
    }
}

impl<M: AccessMesadata> Step<Invoker<M>, String> for ExecutionStepper {
    type Error = InvokerError;

    fn step(&mut self, invoker: &mut Invoker<M>) -> Result<ControlFlow<String>, InvokerError> {
        match self.step_catch_err(&mut invoker.autorouter) {
            Ok(ControlFlow::Continue(())) => Ok(ControlFlow::Continue(())),
            Ok(ControlFlow::Break((maybe_edit, msg))) => {
                if let (Some(command), Some(edit)) = (invoker.ongoing_command.take(), maybe_edit) {
                    invoker.history.do_(command, Some(edit));
                }

                Ok(ControlFlow::Break(msg))
            }
            Err(err) => {
                invoker.ongoing_command = None;
                Err(err)
            }
        }
    }
}
