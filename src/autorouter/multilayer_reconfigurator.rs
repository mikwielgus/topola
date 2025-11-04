// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, ops::ControlFlow};

use specctra_core::mesadata::AccessMesadata;

use crate::{
    autorouter::{
        invoker::GetDebugOverlayData,
        multilayer_autoroute::{
            MultilayerAutorouteConfiguration, MultilayerAutorouteExecutionStepper,
            MultilayerAutorouteOptions,
        },
        multilayer_preconfigurer::{
            MultilayerAutoroutePreconfigurerInput, MultilayerPreconfigurer,
        },
        multilayer_reconfigurer::{
            IncrementFailedRatlineLayersMultilayerAutorouteReconfigurer,
            MakeNextMultilayerAutorouteConfiguration, MultilayerAutorouteReconfigurer,
        },
        planar_autoroute::PlanarAutorouteConfigurationStatus,
        planar_preconfigurer::PlanarAutoroutePreconfigurerInput,
        planar_reconfigurator::PlanarAutorouteReconfiguratorStatus,
        Autorouter, AutorouterError,
    },
    board::edit::BoardEdit,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    router::{navcord::Navcord, navmesh::Navmesh, thetastar::ThetastarStepper},
    stepper::{
        Abort, EstimateProgress, GetMaybeReconfigurationTriggerProgress, LinearScale,
        ReconfiguratorStatus, Reconfigure, SmaRateReconfigurationTrigger, Step,
    },
};

pub type MultilayerReconfiguratorStatus =
    ReconfiguratorStatus<(), PlanarAutorouteReconfiguratorStatus>;

pub struct MultilayerAutorouteReconfigurator {
    stepper: MultilayerAutorouteExecutionStepper,
    reconfiguration_trigger: SmaRateReconfigurationTrigger,
    reconfigurer: MultilayerAutorouteReconfigurer,
    options: MultilayerAutorouteOptions,
}

impl MultilayerAutorouteReconfigurator {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        input: MultilayerAutoroutePreconfigurerInput,
        options: MultilayerAutorouteOptions,
    ) -> Result<Self, AutorouterError> {
        let preconfigurer = MultilayerPreconfigurer::new(autorouter, input.clone());

        let preconfiguration = MultilayerAutorouteConfiguration {
            plan: preconfigurer.plan().clone(),
            planar: PlanarAutoroutePreconfigurerInput {
                ratlines: input.ratlines.clone(),
                terminating_dot_map: BTreeMap::new(),
            },
        };
        let reconfigurer = MultilayerAutorouteReconfigurer::UniformRandomLayers(
            IncrementFailedRatlineLayersMultilayerAutorouteReconfigurer::new(
                autorouter,
                preconfiguration.clone(),
                &options,
            ),
        );

        Ok(Self {
            stepper: MultilayerAutorouteExecutionStepper::new(
                autorouter,
                preconfiguration,
                options,
            )?,
            reconfiguration_trigger: SmaRateReconfigurationTrigger::new(4, 0.5, 0.5),
            reconfigurer,
            options,
        })
    }

    fn reconfigure<M: AccessMesadata>(
        &mut self,
        autorouter: &mut Autorouter<M>,
        planar_result: Result<PlanarAutorouteConfigurationStatus, AutorouterError>,
    ) -> Result<ControlFlow<Option<BoardEdit>, MultilayerReconfiguratorStatus>, AutorouterError>
    {
        // Reset the reconfiguration trigger.
        self.reconfiguration_trigger = SmaRateReconfigurationTrigger::new(4, 0.5, 0.5);

        loop {
            self.reconfigurer
                .process_planar_result(autorouter, planar_result.clone());

            let configuration = match self.reconfigurer.next_configuration(autorouter) {
                ControlFlow::Continue(()) => {
                    return Ok(ControlFlow::Continue(ReconfiguratorStatus::Running(
                        ReconfiguratorStatus::Reconfigured(planar_result?),
                    )))
                }
                ControlFlow::Break(None) => return Ok(ControlFlow::Break(None)),
                ControlFlow::Break(Some(configuration)) => configuration,
            };

            match self.stepper.reconfigure(autorouter, configuration) {
                Ok(_) => {
                    return Ok(ControlFlow::Continue(
                        ReconfiguratorStatus::Reconfigured(()),
                    ))
                }
                Err(AutorouterError::NothingToUndoForReconfiguration) => continue,
                Err(err) => return Err(err),
            }
        }
    }
}

impl<M: AccessMesadata> Step<Autorouter<M>, Option<BoardEdit>, MultilayerReconfiguratorStatus>
    for MultilayerAutorouteReconfigurator
{
    type Error = AutorouterError;

    fn step(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<ControlFlow<Option<BoardEdit>, MultilayerReconfiguratorStatus>, AutorouterError>
    {
        self.reconfiguration_trigger
            .update(*self.estimate_progress().value() as f64);

        match self.stepper.step(autorouter) {
            Ok(ControlFlow::Break(maybe_edit)) => Ok(ControlFlow::Break(maybe_edit)),
            Ok(ControlFlow::Continue(ReconfiguratorStatus::Running(status))) => {
                Ok(ControlFlow::Continue(ReconfiguratorStatus::Running(
                    ReconfiguratorStatus::Running(status),
                )))
            }
            Ok(ControlFlow::Continue(ReconfiguratorStatus::Reconfigured(status))) => {
                self.reconfigure(autorouter, Ok(status))
            }
            Err(err) => self.reconfigure(autorouter, Err(err)),
        }
    }
}

impl<M: AccessMesadata> Abort<Autorouter<M>> for MultilayerAutorouteReconfigurator {
    fn abort(&mut self, autorouter: &mut Autorouter<M>) {
        self.stepper.abort(autorouter)
    }
}

impl EstimateProgress for MultilayerAutorouteReconfigurator {
    type Value = usize;
    type Subscale = LinearScale<f64>;

    fn estimate_progress(&self) -> LinearScale<usize, LinearScale<f64>> {
        self.stepper.estimate_progress()
    }
}

impl GetMaybeReconfigurationTriggerProgress for MultilayerAutorouteReconfigurator {
    type Subscale = ();

    fn reconfiguration_trigger_progress(&self) -> Option<LinearScale<f64>> {
        Some(LinearScale::new(
            (*self.reconfiguration_trigger.maybe_sma_rate_per_sec())?,
            *self.reconfiguration_trigger.min_sma_rate_per_sec(),
            (),
        ))
    }
}

impl GetDebugOverlayData for MultilayerAutorouteReconfigurator {
    fn maybe_thetastar(&self) -> Option<&ThetastarStepper<Navmesh, f64>> {
        self.stepper.maybe_thetastar()
    }

    fn maybe_navcord(&self) -> Option<&Navcord> {
        self.stepper.maybe_navcord()
    }

    fn ghosts(&self) -> &[PrimitiveShape] {
        self.stepper.ghosts()
    }

    fn obstacles(&self) -> &[PrimitiveIndex] {
        self.stepper.obstacles()
    }
}
