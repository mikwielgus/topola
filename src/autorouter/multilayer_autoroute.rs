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
        planar_autoroute::PlanarAutorouteContinueStatus,
        planar_reconfigurator::PlanarAutorouteExecutionReconfigurator,
        ratline::RatlineUid,
        Autorouter, AutorouterError, PlanarAutorouteOptions,
    },
    board::edit::BoardEdit,
    drawing::graph::PrimitiveIndex,
    geometry::{edit::Edit, primitive::PrimitiveShape},
    router::{navcord::Navcord, navmesh::Navmesh, thetastar::ThetastarStepper},
    stepper::{Abort, EstimateProgress, ReconfiguratorStatus, Step},
};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct MultilayerAutorouteOptions {
    pub anterouter: AnterouterOptions,
    pub planar: PlanarAutorouteOptions,
}

pub struct MultilayerAutorouteExecutionStepper {
    planar: PlanarAutorouteExecutionReconfigurator,
    anteroute_edit: BoardEdit,
}

impl MultilayerAutorouteExecutionStepper {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratlines: Vec<RatlineUid>,
        plan: AnterouterPlan,
        options: MultilayerAutorouteOptions,
    ) -> Result<Self, AutorouterError> {
        let mut assigner = Anterouter::new(plan);
        let mut anteroute_edit = BoardEdit::new();
        assigner.anteroute(autorouter, &mut anteroute_edit, &options.anterouter);

        Ok(Self {
            planar: PlanarAutorouteExecutionReconfigurator::new(
                autorouter,
                ratlines,
                options.planar,
            )?,
            anteroute_edit,
        })
    }
}

impl<M: AccessMesadata>
    Step<Autorouter<M>, Option<BoardEdit>, ReconfiguratorStatus<(), PlanarAutorouteContinueStatus>>
    for MultilayerAutorouteExecutionStepper
{
    type Error = AutorouterError;

    fn step(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<
        ControlFlow<Option<BoardEdit>, ReconfiguratorStatus<(), PlanarAutorouteContinueStatus>>,
        AutorouterError,
    > {
        match self.planar.step(autorouter) {
            Ok(ControlFlow::Break(Some(edit))) => {
                self.anteroute_edit.merge(edit);
                // FIXME: Unnecessary large clone.
                Ok(ControlFlow::Break(Some(self.anteroute_edit.clone())))
            }
            x => x,
        }
    }
}

impl<M: AccessMesadata> Abort<Autorouter<M>> for MultilayerAutorouteExecutionStepper {
    fn abort(&mut self, autorouter: &mut Autorouter<M>) {
        self.planar.abort(autorouter)
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
