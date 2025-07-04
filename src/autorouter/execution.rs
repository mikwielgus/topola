// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{collections::BTreeSet, ops::ControlFlow};

use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use crate::{
    board::AccessMesadata,
    layout::{via::ViaWeight, LayoutEdit},
    router::ng,
    stepper::{Abort, Step},
};

use super::{
    autoroute::AutorouteExecutionStepper,
    compare_detours::CompareDetoursExecutionStepper,
    invoker::{GetDebugOverlayData, Invoker, InvokerError},
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
    TopoAutoroute {
        selection: PinSelection,
        #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
        allowed_edges: BTreeSet<ng::PieEdgeIndex>,
        active_layer: String,
        routed_band_width: f64,
    },
    PlaceVia(ViaWeight),
    RemoveBands(BandSelection),
    CompareDetours(Type, AutorouterOptions),
    MeasureLength(BandSelection),
}

#[enum_dispatch(GetDebugOverlayData)]
pub enum ExecutionStepper<M> {
    Autoroute(AutorouteExecutionStepper),
    TopoAutoroute(ng::AutorouteExecutionStepper<M>),
    PlaceVia(PlaceViaExecutionStepper),
    RemoveBands(RemoveBandsExecutionStepper),
    CompareDetours(CompareDetoursExecutionStepper),
    MeasureLength(MeasureLengthExecutionStepper),
}

impl<M: AccessMesadata + Clone> ExecutionStepper<M> {
    fn step_catch_err(
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
            ExecutionStepper::TopoAutoroute(autoroute) => {
                let ret = match autoroute.step() {
                    ControlFlow::Continue(()) => ControlFlow::Continue(()),
                    ControlFlow::Break(false) => {
                        ControlFlow::Break((None, "topo-autorouting failed".to_string()))
                    }
                    ControlFlow::Break(true) => {
                        for (ep, band) in &autoroute.last_bands {
                            let (source, target) = ep.end_points.into();
                            autorouter
                                .board
                                .try_set_band_between_nodes(source, target, *band);
                        }

                        let topo_navmesh = autoroute.maybe_topo_navmesh().unwrap().to_owned();
                        let mut pretty_config = ron::ser::PrettyConfig::new();
                        pretty_config.depth_limit = 2;
                        log::debug!(
                            "topo navmesh result: {}",
                            ron::ser::to_string_pretty(
                                &ng::pie::navmesh::NavmeshSer::from(topo_navmesh),
                                pretty_config
                            )
                            .unwrap()
                        );

                        ControlFlow::Break((
                            Some(autoroute.last_recorder.clone()),
                            "finished topo-autorouting".to_string(),
                        ))
                    }
                };
                // TODO: maintain topo-navmesh just like layout
                *autorouter.board.layout_mut() = autoroute.last_layout.clone();
                ret
            }
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

impl<M: AccessMesadata + Clone> Step<Invoker<M>, String> for ExecutionStepper<M> {
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

impl<M: AccessMesadata + Clone> Abort<Invoker<M>> for ExecutionStepper<M> {
    fn abort(&mut self, invoker: &mut Invoker<M>) {
        match self {
            ExecutionStepper::TopoAutoroute(autoroute) => {
                autoroute.abort(&mut ());
                // TODO: maintain topo-navmesh just like layout
                *invoker.autorouter.board.layout_mut() = autoroute.last_layout.clone();
            }
            execution => {
                // TODO
                execution.finish(invoker);
            }
        }
    }
}
