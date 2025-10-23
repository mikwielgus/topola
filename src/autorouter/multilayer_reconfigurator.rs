// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::ops::ControlFlow;

use specctra_core::mesadata::AccessMesadata;

use crate::{
    autorouter::{
        invoker::GetDebugOverlayData,
        multilayer_autoroute::{
            MultilayerAutorouteConfiguration, MultilayerAutorouteExecutionStepper,
            MultilayerAutorouteOptions,
        },
        multilayer_reconfigurer::MultilayerReconfigurer,
        planar_reconfigurator::{PlanarAutorouteReconfiguratorInput, PlanarReconfiguratorStatus},
        planner::Planner,
        ratline::RatlineUid,
        Autorouter, AutorouterError,
    },
    board::edit::BoardEdit,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    router::{navcord::Navcord, navmesh::Navmesh, thetastar::ThetastarStepper},
    stepper::{Abort, EstimateProgress, ReconfiguratorStatus, Reconfigure, Step},
};

pub struct MultilayerAutorouteReconfiguratorInput {
    pub ratlines: Vec<RatlineUid>,
}

pub type MultilayerReconfiguratorStatus = ReconfiguratorStatus<(), PlanarReconfiguratorStatus>;

pub struct MultilayerAutorouteReconfigurator {
    stepper: MultilayerAutorouteExecutionStepper,
    reconfigurer: MultilayerReconfigurer,
    options: MultilayerAutorouteOptions,
    // TODO: Obviously, we need something more sophisticated here.
    planar_autoroute_reconfiguration_count: u64,
}

impl MultilayerAutorouteReconfigurator {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        input: MultilayerAutorouteReconfiguratorInput,
        options: MultilayerAutorouteOptions,
    ) -> Result<Self, AutorouterError> {
        let planner = Planner::new(autorouter, &input.ratlines);
        let preconfiguration = MultilayerAutorouteConfiguration {
            plan: planner.plan().clone(),
            planar: PlanarAutorouteReconfiguratorInput {
                ratlines: input.ratlines.clone(),
            },
        };
        let reconfigurer = MultilayerReconfigurer::new(autorouter, input.ratlines, &options);

        Ok(Self {
            stepper: MultilayerAutorouteExecutionStepper::new(
                autorouter,
                preconfiguration,
                options,
            )?,
            reconfigurer,
            options,
            planar_autoroute_reconfiguration_count: 0,
        })
    }

    fn reconfigure<M: AccessMesadata>(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<ControlFlow<Option<BoardEdit>, MultilayerReconfiguratorStatus>, AutorouterError>
    {
        loop {
            let Some(plan) = self.reconfigurer.next_configuration(autorouter) else {
                return Ok(ControlFlow::Break(None));
            };

            match self.stepper.reconfigure(autorouter, plan) {
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
        match self.stepper.step(autorouter) {
            Ok(ControlFlow::Break(maybe_edit)) => Ok(ControlFlow::Break(maybe_edit)),
            Ok(ControlFlow::Continue(ReconfiguratorStatus::Running(status))) => {
                Ok(ControlFlow::Continue(ReconfiguratorStatus::Running(
                    ReconfiguratorStatus::Running(status),
                )))
            }
            Ok(ControlFlow::Continue(ReconfiguratorStatus::Reconfigured(status))) => {
                self.planar_autoroute_reconfiguration_count += 1;

                if self.planar_autoroute_reconfiguration_count >= 100 {
                    self.planar_autoroute_reconfiguration_count = 0;
                    self.reconfigure(autorouter)
                } else {
                    Ok(ControlFlow::Continue(ReconfiguratorStatus::Running(
                        ReconfiguratorStatus::Reconfigured(status),
                    )))
                }
            }
            Err(_) => self.reconfigure(autorouter),
        }
    }
}

impl<M: AccessMesadata> Abort<Autorouter<M>> for MultilayerAutorouteReconfigurator {
    fn abort(&mut self, autorouter: &mut Autorouter<M>) {
        self.stepper.abort(autorouter)
    }
}

impl EstimateProgress for MultilayerAutorouteReconfigurator {
    type Value = f64;

    fn estimate_progress_value(&self) -> f64 {
        // TODO.
        self.stepper.estimate_progress_value()
    }

    fn estimate_progress_maximum(&self) -> f64 {
        // TODO.
        self.stepper.estimate_progress_maximum()
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
