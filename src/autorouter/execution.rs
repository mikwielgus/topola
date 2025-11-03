// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{collections::BTreeSet, ops::ControlFlow};

use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use crate::{
    autorouter::{
        multilayer_autoroute::MultilayerAutorouteOptions,
        multilayer_reconfigurator::MultilayerAutorouteReconfigurator,
        planar_reconfigurator::PlanarAutorouteReconfigurator,
    },
    board::{edit::BoardEdit, AccessMesadata},
    layout::via::ViaWeight,
    router::ng,
    stepper::{Abort, EstimateProgress, GetMaybeReconfigurationTriggerProgress, LinearScale, Step},
};

use super::{
    invoker::{GetDebugOverlayData, Invoker, InvokerError},
    measure_length::MeasureLengthExecutionStepper,
    place_via::PlaceViaExecutionStepper,
    remove_bands::RemoveBandsExecutionStepper,
    selection::{BandSelection, PinSelection},
    Autorouter, PlanarAutorouteOptions,
};

type Type = PinSelection;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    Autoroute(PinSelection, PlanarAutorouteOptions), // TODO: Rename to PlanarAutoroute.
    MultilayerAutoroute(PinSelection, MultilayerAutorouteOptions),
    TopoAutoroute {
        selection: PinSelection,
        #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
        allowed_edges: BTreeSet<ng::PieEdgeIndex>,
        active_layer: String,
        routed_band_width: f64,
    },
    PlaceVia(ViaWeight),
    RemoveBands(BandSelection),
    MeasureLength(BandSelection),
}

#[enum_dispatch(GetDebugOverlayData)]
pub enum ExecutionStepper<M> {
    MultilayerAutoroute(MultilayerAutorouteReconfigurator),
    PlanarAutoroute(PlanarAutorouteReconfigurator),
    TopoAutoroute(ng::AutorouteExecutionStepper<M>),
    PlaceVia(PlaceViaExecutionStepper),
    RemoveBands(RemoveBandsExecutionStepper),
    MeasureLength(MeasureLengthExecutionStepper),
}

impl<M: AccessMesadata + Clone> ExecutionStepper<M> {
    fn step_catch_err(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<ControlFlow<(Option<BoardEdit>, String)>, InvokerError> {
        Ok(match self {
            ExecutionStepper::MultilayerAutoroute(autoroute) => match autoroute.step(autorouter)? {
                ControlFlow::Continue(..) => ControlFlow::Continue(()),
                ControlFlow::Break(edit) => {
                    ControlFlow::Break((edit, "finished multilayer autorouting".to_string()))
                }
            },
            ExecutionStepper::PlanarAutoroute(autoroute) => match autoroute.step(autorouter)? {
                ControlFlow::Continue(..) => ControlFlow::Continue(()),
                ControlFlow::Break(edit) => {
                    ControlFlow::Break((edit, "finished planar autorouting".to_string()))
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
                            autorouter.board.try_set_band_between_nodes(
                                &mut autoroute.last_recorder.board_data_edit,
                                source,
                                target,
                                *band,
                            );
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
            ExecutionStepper::MultilayerAutoroute(autoroute) => {
                autoroute.abort(&mut invoker.autorouter)
            }
            ExecutionStepper::PlanarAutoroute(autoroute) => {
                autoroute.abort(&mut invoker.autorouter)
            }
            ExecutionStepper::TopoAutoroute(autoroute) => {
                autoroute.abort(&mut ());
                // TODO: maintain topo-navmesh just like layout
                *invoker.autorouter.board.layout_mut() = autoroute.last_layout.clone();
            }
            ExecutionStepper::PlaceVia(_place_via) => (), //place_via.abort(),
            ExecutionStepper::RemoveBands(_remove_bands) => (), //remove_bands.abort(),
            ExecutionStepper::MeasureLength(_measure_length) => (), //measure_length.abort(),
        }
    }
}

// Since enum_dispatch does not really support generics, we implement this the
// long way by using `match`.
impl<M> EstimateProgress for ExecutionStepper<M> {
    type Value = usize;
    type Subscale = LinearScale<f64>;

    fn estimate_progress(&self) -> LinearScale<usize, LinearScale<f64>> {
        match self {
            ExecutionStepper::MultilayerAutoroute(autoroute) => autoroute.estimate_progress(),
            ExecutionStepper::PlanarAutoroute(autoroute) => autoroute.estimate_progress(),
            ExecutionStepper::TopoAutoroute(..) => {
                LinearScale::new(0, 0, LinearScale::new(0.0, 0.0, ()))
            }
            ExecutionStepper::PlaceVia(..) => {
                LinearScale::new(0, 0, LinearScale::new(0.0, 0.0, ()))
            }
            ExecutionStepper::RemoveBands(..) => {
                LinearScale::new(0, 0, LinearScale::new(0.0, 0.0, ()))
            }
            ExecutionStepper::MeasureLength(..) => {
                LinearScale::new(0, 0, LinearScale::new(0.0, 0.0, ()))
            }
        }
    }
}

// Since enum_dispatch does not really support generics, we implement this the
// long way by using `match`.
impl<M> GetMaybeReconfigurationTriggerProgress for ExecutionStepper<M> {
    type Subscale = ();

    fn reconfiguration_trigger_progress(&self) -> Option<LinearScale<f64>> {
        match self {
            ExecutionStepper::MultilayerAutoroute(autoroute) => {
                autoroute.reconfiguration_trigger_progress()
            }
            ExecutionStepper::PlanarAutoroute(autoroute) => None,
            ExecutionStepper::TopoAutoroute(..) => None,
            ExecutionStepper::PlaceVia(..) => None,
            ExecutionStepper::RemoveBands(..) => None,
            ExecutionStepper::MeasureLength(..) => None,
        }
    }
}
