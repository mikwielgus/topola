// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::ops::ControlFlow;

use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use specctra_core::mesadata::AccessMesadata;

use crate::{
    autorouter::{
        anterouter::{Anterouter, AnterouterOptions, AnterouterPlan},
        invoker::GetDebugOverlayData,
        planar_preconfigurer::PlanarAutoroutePreconfigurerInput,
        planar_reconfigurator::{
            PlanarAutorouteReconfigurator, PlanarAutorouteReconfiguratorStatus,
        },
        Autorouter, AutorouterError, PlanarAutorouteOptions,
    },
    board::edit::BoardEdit,
    drawing::graph::PrimitiveIndex,
    geometry::{edit::Edit, primitive::PrimitiveShape},
    router::{navcord::Navcord, navmesh::Navmesh, thetastar::ThetastarStepper},
    stepper::{
        Abort, EstimateProgress, LinearScale, ReconfiguratorStatus, Reconfigure, Step,
        TimeoutOptions,
    },
};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MultilayerAutorouteConfiguration {
    pub plan: AnterouterPlan,
    pub planar: PlanarAutoroutePreconfigurerInput,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct MultilayerAutorouteOptions {
    pub anterouter: AnterouterOptions,
    pub planar: PlanarAutorouteOptions,
    pub timeout: TimeoutOptions,
}

#[derive(Getters)]
pub struct MultilayerAutorouteExecutionStepper {
    planar: PlanarAutorouteReconfigurator,
    #[getter(skip)]
    anteroute_edit: BoardEdit,
    #[getter(skip)]
    options: MultilayerAutorouteOptions,
}

impl MultilayerAutorouteExecutionStepper {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        configuration: MultilayerAutorouteConfiguration,
        options: MultilayerAutorouteOptions,
    ) -> Result<Self, AutorouterError> {
        let mut anterouter = Anterouter::new(configuration.plan);
        let mut anteroute_edit = BoardEdit::new();
        let terminating_dot_map =
            anterouter.anteroute(autorouter, &mut anteroute_edit, &options.anterouter);

        Ok(Self {
            planar: PlanarAutorouteReconfigurator::new(
                autorouter,
                PlanarAutoroutePreconfigurerInput {
                    terminating_dot_map,
                    ratlines: configuration.planar.ratlines,
                },
                options.planar,
            )?,
            anteroute_edit,
            options: options.clone(),
        })
    }
}

impl<M: AccessMesadata> Step<Autorouter<M>, Option<BoardEdit>, PlanarAutorouteReconfiguratorStatus>
    for MultilayerAutorouteExecutionStepper
{
    type Error = AutorouterError;

    fn step(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<ControlFlow<Option<BoardEdit>, PlanarAutorouteReconfiguratorStatus>, AutorouterError>
    {
        match self.planar.step(autorouter) {
            Ok(ControlFlow::Break(Some(edit))) => {
                self.anteroute_edit.merge(edit);
                // FIXME: Unnecessary large clone.
                Ok(ControlFlow::Break(Some(self.anteroute_edit.clone())))
            }
            Ok(ControlFlow::Continue(ReconfiguratorStatus::Reconfigured(result))) => Ok(
                ControlFlow::Continue(ReconfiguratorStatus::Reconfigured(result)),
            ),
            x => x,
        }
    }
}

impl<M: AccessMesadata> Abort<Autorouter<M>> for MultilayerAutorouteExecutionStepper {
    fn abort(&mut self, autorouter: &mut Autorouter<M>) {
        self.planar.abort(autorouter)
    }
}

impl<M: AccessMesadata> Reconfigure<Autorouter<M>> for MultilayerAutorouteExecutionStepper {
    type Configuration = MultilayerAutorouteConfiguration;
    type Output = Result<(), AutorouterError>;

    fn reconfigure(
        &mut self,
        autorouter: &mut Autorouter<M>,
        new_configuration: MultilayerAutorouteConfiguration,
    ) -> Result<(), AutorouterError> {
        self.planar.abort(autorouter);

        // FIXME: this somehow corrupts internal state.
        autorouter.board.apply_edit(&self.anteroute_edit.reverse());

        *self = Self::new(autorouter, new_configuration, self.options)?;
        Ok(())
    }
}

impl EstimateProgress for MultilayerAutorouteExecutionStepper {
    type Value = usize;
    type Subscale = LinearScale<f64>;

    fn estimate_progress(&self) -> LinearScale<usize, LinearScale<f64>> {
        self.planar.estimate_progress()
    }
}

impl GetDebugOverlayData for MultilayerAutorouteExecutionStepper {
    fn maybe_thetastar(&self) -> Option<&ThetastarStepper<Navmesh, f64>> {
        self.planar.maybe_thetastar()
    }

    fn maybe_navcord(&self) -> Option<&Navcord> {
        self.planar.maybe_navcord()
    }

    fn ghosts(&self) -> &[PrimitiveShape] {
        self.planar.ghosts()
    }

    fn obstacles(&self) -> &[PrimitiveIndex] {
        self.planar.obstacles()
    }
}
