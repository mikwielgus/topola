// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, ops::ControlFlow};

use enum_dispatch::enum_dispatch;
use specctra_core::mesadata::AccessMesadata;

use crate::autorouter::{
    multilayer_autoroute::{MultilayerAutorouteConfiguration, MultilayerAutorouteOptions},
    planar_autoroute::PlanarAutorouteConfigurationStatus,
    planar_preconfigurer::PlanarAutoroutePreconfigurerInput,
    Autorouter, AutorouterError,
};

#[enum_dispatch]
pub trait MakeNextMultilayerAutorouteConfiguration {
    fn next_configuration(
        &mut self,
        autorouter: &Autorouter<impl AccessMesadata>,
        planar_result: Result<PlanarAutorouteConfigurationStatus, AutorouterError>,
    ) -> ControlFlow<Option<MultilayerAutorouteConfiguration>>;
}

#[enum_dispatch(MakeNextMultilayerAutorouteConfiguration)]
pub enum MultilayerAutorouteReconfigurer {
    UniformRandomLayers(IncrementFailedRatlineLayersMultilayerAutorouteReconfigurer),
}

pub struct IncrementFailedRatlineLayersMultilayerAutorouteReconfigurer {
    last_configuration: MultilayerAutorouteConfiguration,
    maybe_best_planar_status: Option<PlanarAutorouteConfigurationStatus>,
    planar_autoroute_reconfiguration_count: u64,
}

impl IncrementFailedRatlineLayersMultilayerAutorouteReconfigurer {
    pub fn new(
        _autorouter: &Autorouter<impl AccessMesadata>,
        preconfiguration: MultilayerAutorouteConfiguration,
        _options: &MultilayerAutorouteOptions,
    ) -> Self {
        Self {
            last_configuration: preconfiguration,
            maybe_best_planar_status: None,
            planar_autoroute_reconfiguration_count: 0,
        }
    }
}

impl MakeNextMultilayerAutorouteConfiguration
    for IncrementFailedRatlineLayersMultilayerAutorouteReconfigurer
{
    fn next_configuration(
        &mut self,
        autorouter: &Autorouter<impl AccessMesadata>,
        planar_result: Result<PlanarAutorouteConfigurationStatus, AutorouterError>,
    ) -> ControlFlow<Option<MultilayerAutorouteConfiguration>> {
        self.planar_autoroute_reconfiguration_count += 1;

        let Ok(planar_status) = planar_result else {
            return ControlFlow::Break(None);
        };

        if self
            .maybe_best_planar_status
            .as_ref()
            .is_none_or(|status| status.costs.lengths.len() < planar_status.costs.lengths.len())
        {
            self.maybe_best_planar_status = Some(planar_status.clone());
        }

        if self.planar_autoroute_reconfiguration_count < 10 {
            return ControlFlow::Continue(());
        }

        self.planar_autoroute_reconfiguration_count = 0;

        let mut new_anterouter_plan = self.last_configuration.plan.clone();

        if let Some(ref best_planar_status) = self.maybe_best_planar_status {
            for ratline_index in
                best_planar_status.costs.lengths.len()..planar_status.configuration.ratlines.len()
            {
                *new_anterouter_plan
                    .layer_map
                    .get_mut(&planar_status.configuration.ratlines[ratline_index])
                    .unwrap() += 1;
                *new_anterouter_plan
                    .layer_map
                    .get_mut(&planar_status.configuration.ratlines[ratline_index])
                    .unwrap() %= autorouter.board().layout().drawing().layer_count();
            }
        }

        self.last_configuration = MultilayerAutorouteConfiguration {
            plan: new_anterouter_plan,
            planar: PlanarAutoroutePreconfigurerInput {
                ratlines: self.last_configuration.planar.ratlines.clone(),
                terminating_dot_map: BTreeMap::new(),
            },
        };

        ControlFlow::Break(Some(self.last_configuration.clone()))
    }
}
