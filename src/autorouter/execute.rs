use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use crate::{
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    layout::via::ViaWeight,
    router::{navmesh::Navmesh, trace::Trace},
    step::Step,
};

use super::{
    autoroute::{Autoroute, AutorouteStatus},
    compare_detours::{CompareDetours, CompareDetoursStatus},
    invoker::{
        GetGhosts, GetMaybeNavmesh, GetMaybeTrace, GetObstacles, Invoker, InvokerError,
        InvokerStatus,
    },
    measure_length::MeasureLength,
    place_via::PlaceVia,
    remove_bands::RemoveBands,
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
pub enum Execute {
    Autoroute(Autoroute),
    PlaceVia(PlaceVia),
    RemoveBands(RemoveBands),
    CompareDetours(CompareDetours),
    MeasureLength(MeasureLength),
}

impl Execute {
    fn step_catch_err<M: AccessMesadata>(
        &mut self,
        invoker: &mut Invoker<M>,
    ) -> Result<InvokerStatus, InvokerError> {
        match self {
            Execute::Autoroute(autoroute) => match autoroute.step(&mut invoker.autorouter)? {
                AutorouteStatus::Running => Ok(InvokerStatus::Running),
                AutorouteStatus::Routed(..) => Ok(InvokerStatus::Running),
                AutorouteStatus::Finished => Ok(InvokerStatus::Finished(String::from(
                    "finished autorouting",
                ))),
            },
            Execute::PlaceVia(place_via) => {
                place_via.doit(&mut invoker.autorouter)?;
                Ok(InvokerStatus::Finished(String::from(
                    "finished placing via",
                )))
            }
            Execute::RemoveBands(remove_bands) => {
                remove_bands.doit(&mut invoker.autorouter)?;
                Ok(InvokerStatus::Finished(String::from(
                    "finished removing bands",
                )))
            }
            Execute::CompareDetours(compare_detours) => {
                match compare_detours.step(&mut invoker.autorouter)? {
                    CompareDetoursStatus::Running => Ok(InvokerStatus::Running),
                    CompareDetoursStatus::Finished(total_length1, total_length2) => {
                        Ok(InvokerStatus::Finished(String::from(format!(
                            "total detour lengths are {} and {}",
                            total_length1, total_length2
                        ))))
                    }
                }
            }
            Execute::MeasureLength(measure_length) => {
                let length = measure_length.doit(&mut invoker.autorouter)?;
                Ok(InvokerStatus::Finished(format!(
                    "Total length of selected bands: {}",
                    length
                )))
            }
        }
    }
}

impl<M: AccessMesadata> Step<Invoker<M>, InvokerStatus, InvokerError, ()> for Execute {
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
