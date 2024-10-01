use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use crate::{board::mesadata::AccessMesadata, layout::via::ViaWeight, stepper::Step};

use super::{
    autoroute::{AutorouteExecutionStepper, AutorouteStatus},
    compare_detours::{CompareDetoursExecutionStepper, CompareDetoursStatus},
    invoker::{Invoker, InvokerError, InvokerStatus},
    measure_length::MeasureLengthExecutionStepper,
    place_via::PlaceViaExecutionStepper,
    remove_bands::RemoveBandsExecutionStepper,
    selection::{BandSelection, PinSelection},
    AutorouterOptions,
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

#[enum_dispatch(GetMaybeNavmesh, GetMaybeTrace, GetGhosts, GetObstacles)]
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
        invoker: &mut Invoker<M>,
    ) -> Result<InvokerStatus, InvokerError> {
        Ok(match self {
            ExecutionStepper::Autoroute(autoroute) => {
                match autoroute.step(&mut invoker.autorouter)? {
                    AutorouteStatus::Running => InvokerStatus::Running,
                    AutorouteStatus::Routed(..) => InvokerStatus::Running,
                    AutorouteStatus::Finished => InvokerStatus::Finished(
                        "finished autorouting".to_string(),
                    ),
                }
            }
            ExecutionStepper::PlaceVia(place_via) => {
                place_via.doit(&mut invoker.autorouter)?;
                InvokerStatus::Finished("finished placing via".to_string())
            }
            ExecutionStepper::RemoveBands(remove_bands) => {
                remove_bands.doit(&mut invoker.autorouter)?;
                InvokerStatus::Finished("finished removing bands".to_string())
            }
            ExecutionStepper::CompareDetours(compare_detours) => {
                match compare_detours.step(&mut invoker.autorouter)? {
                    CompareDetoursStatus::Running => InvokerStatus::Running,
                    CompareDetoursStatus::Finished(total_length1, total_length2) => {
                        InvokerStatus::Finished(format!(
                            "total detour lengths are {} and {}",
                            total_length1, total_length2
                        ))
                    }
                }
            }
            ExecutionStepper::MeasureLength(measure_length) => {
                let length = measure_length.doit(&mut invoker.autorouter)?;
                InvokerStatus::Finished(format!(
                    "Total length of selected bands: {}",
                    length
                ))
            }
        })
    }
}

impl<M: AccessMesadata> Step<Invoker<M>, InvokerStatus, InvokerError, ()> for ExecutionStepper {
    fn step(&mut self, invoker: &mut Invoker<M>) -> Result<InvokerStatus, InvokerError> {
        match self.step_catch_err(invoker) {
            Ok(InvokerStatus::Running) => Ok(InvokerStatus::Running),
            Ok(InvokerStatus::Finished(msg)) => {
                if let Some(command) = invoker.ongoing_command.take() {
                    invoker.history.do_(command);
                }

                Ok(InvokerStatus::Finished(msg))
            }
            Err(err) => {
                invoker.ongoing_command = None;
                Err(err)
            }
        }
    }
}
