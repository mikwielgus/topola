// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::ops::ControlFlow;

use serde::{Deserialize, Serialize};
use specctra_core::mesadata::AccessMesadata;

use crate::{
    autorouter::{
        anterouter::{Anterouter, AnterouterOptions, AnterouterPlan},
        invoker::GetDebugOverlayData,
        planar_reconfigurator::{
            PlanarAutorouteReconfigurator, PlanarAutorouteReconfiguratorInput,
            PlanarReconfiguratorStatus,
        },
        Autorouter, AutorouterError, PlanarAutorouteOptions,
    },
    board::edit::BoardEdit,
    drawing::graph::PrimitiveIndex,
    geometry::{edit::Edit, primitive::PrimitiveShape},
    router::{navcord::Navcord, navmesh::Navmesh, thetastar::ThetastarStepper},
    stepper::{Abort, EstimateProgress, ReconfiguratorStatus, Reconfigure, Step},
};

#[derive(Clone, Debug)]
pub struct MultilayerAutorouteConfiguration {
    pub plan: AnterouterPlan,
    pub planar: PlanarAutorouteReconfiguratorInput,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct MultilayerAutorouteOptions {
    pub anterouter: AnterouterOptions,
    pub planar: PlanarAutorouteOptions,
}

pub struct MultilayerAutorouteExecutionStepper {
    planar: PlanarAutorouteReconfigurator,
    anteroute_edit: BoardEdit,
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
        anterouter.anteroute(autorouter, &mut anteroute_edit, &options.anterouter);

        Ok(Self {
            planar: PlanarAutorouteReconfigurator::new(
                autorouter,
                configuration.planar,
                options.planar,
            )?,
            anteroute_edit,
            options: options.clone(),
        })
    }
}

impl<M: AccessMesadata> Step<Autorouter<M>, Option<BoardEdit>, PlanarReconfiguratorStatus>
    for MultilayerAutorouteExecutionStepper
{
    type Error = AutorouterError;

    fn step(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<ControlFlow<Option<BoardEdit>, PlanarReconfiguratorStatus>, AutorouterError> {
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
    type Configuration = AnterouterPlan;
    type Output = Result<(), AutorouterError>;

    fn reconfigure(
        &mut self,
        autorouter: &mut Autorouter<M>,
        plan: AnterouterPlan,
    ) -> Result<(), AutorouterError> {
        self.planar.abort(autorouter);
        autorouter.board.apply_edit(&self.anteroute_edit.reverse());

        let mut anterouter = Anterouter::new(plan);
        let mut anteroute_edit = BoardEdit::new();
        anterouter.anteroute(autorouter, &mut anteroute_edit, &self.options.anterouter);
        Ok(())
    }
}

impl EstimateProgress for MultilayerAutorouteExecutionStepper {
    type Value = f64;

    fn estimate_progress_value(&self) -> f64 {
        self.planar.estimate_progress_value()
    }

    fn estimate_progress_maximum(&self) -> f64 {
        self.planar.estimate_progress_maximum()
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
