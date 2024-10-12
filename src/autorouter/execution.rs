use std::ops::ControlFlow;

use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use crate::{board::mesadata::AccessMesadata, layout::via::ViaWeight, stepper::Step};

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

#[enum_dispatch(GetMaybeNavmesh, GetMaybeNavcord, GetGhosts, GetObstacles)]
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
    ) -> Result<ControlFlow<String>, InvokerError> {
        Ok(match self {
            ExecutionStepper::Autoroute(autoroute) => match autoroute.step(autorouter)? {
                ControlFlow::Continue(..) => ControlFlow::Continue(()),
                ControlFlow::Break(..) => ControlFlow::Break("finished autorouting".to_string()),
            },
            ExecutionStepper::PlaceVia(place_via) => {
                place_via.doit(autorouter)?;
                ControlFlow::Break("finished placing via".to_string())
            }
            ExecutionStepper::RemoveBands(remove_bands) => {
                remove_bands.doit(autorouter)?;
                ControlFlow::Break("finished removing bands".to_string())
            }
            ExecutionStepper::CompareDetours(compare_detours) => {
                match compare_detours.step(autorouter)? {
                    ControlFlow::Continue(()) => ControlFlow::Continue(()),
                    ControlFlow::Break((total_length1, total_length2)) => {
                        ControlFlow::Break(format!(
                            "total detour lengths are {} and {}",
                            total_length1, total_length2
                        ))
                    }
                }
            }
            ExecutionStepper::MeasureLength(measure_length) => {
                let length = measure_length.doit(autorouter)?;
                ControlFlow::Break(format!("Total length of selected bands: {}", length))
            }
        })
    }
}

impl<M: AccessMesadata> Step<Invoker<M>, String> for ExecutionStepper {
    type Error = InvokerError;

    fn step(&mut self, invoker: &mut Invoker<M>) -> Result<ControlFlow<String>, InvokerError> {
        match self.step_catch_err(&mut invoker.autorouter) {
            Ok(ControlFlow::Continue(())) => Ok(ControlFlow::Continue(())),
            Ok(ControlFlow::Break(msg)) => {
                if let Some(command) = invoker.ongoing_command.take() {
                    invoker.history.do_(command);
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
