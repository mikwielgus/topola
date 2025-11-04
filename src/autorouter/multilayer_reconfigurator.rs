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
            reconfiguration_trigger: SmaRateReconfigurationTrigger::new(20, 0.5, 0.1),
            reconfigurer,
            options,
        })
    }

    fn reconfigure(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
    ) -> Result<ControlFlow<Option<BoardEdit>, MultilayerReconfiguratorStatus>, AutorouterError>
    {
        // Reset the reconfiguration trigger.
        self.reconfiguration_trigger = SmaRateReconfigurationTrigger::new(20, 1.0, 0.1);

        loop {
            let Some(configuration) = self.reconfigurer.next_configuration(autorouter) else {
                return Ok(ControlFlow::Break(None));
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
        if !self
            .reconfiguration_trigger
            .update(*self.estimate_progress().value() as f64)
        {
            return self.reconfigure(autorouter);
        }

        match self.stepper.step(autorouter) {
            Ok(ControlFlow::Break(maybe_edit)) => Ok(ControlFlow::Break(maybe_edit)),
            Ok(ControlFlow::Continue(ReconfiguratorStatus::Running(status))) => {
                Ok(ControlFlow::Continue(ReconfiguratorStatus::Running(
                    ReconfiguratorStatus::Running(status),
                )))
            }
            Ok(ControlFlow::Continue(ReconfiguratorStatus::Reconfigured(status))) => {
                self.reconfigurer
                    .process_planar_result(autorouter, Ok(status));
                Ok(ControlFlow::Continue(
                    ReconfiguratorStatus::Reconfigured(()),
                ))
            }
            Err(err) => {
                self.reconfigurer
                    .process_planar_result(autorouter, Err(err.clone()));
                self.reconfigure(autorouter)
            }
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
