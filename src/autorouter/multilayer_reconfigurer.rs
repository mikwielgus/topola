// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

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
    fn process_planar_result(
        &mut self,
        autorouter: &Autorouter<impl AccessMesadata>,
        planar_result: Result<PlanarAutorouteConfigurationStatus, AutorouterError>,
    );

    fn next_configuration(
        &mut self,
        autorouter: &Autorouter<impl AccessMesadata>,
    ) -> Option<MultilayerAutorouteConfiguration>;
}

#[enum_dispatch(MakeNextMultilayerAutorouteConfiguration)]
pub enum MultilayerAutorouteReconfigurer {
    UniformRandomLayers(IncrementFailedRatlineLayersMultilayerAutorouteReconfigurer),
}

pub struct IncrementFailedRatlineLayersMultilayerAutorouteReconfigurer {
    last_configuration: MultilayerAutorouteConfiguration,
    maybe_last_planar_status: Option<PlanarAutorouteConfigurationStatus>,
    maybe_best_planar_status: Option<PlanarAutorouteConfigurationStatus>,
}

impl IncrementFailedRatlineLayersMultilayerAutorouteReconfigurer {
    pub fn new(
        _autorouter: &Autorouter<impl AccessMesadata>,
        preconfiguration: MultilayerAutorouteConfiguration,
        _options: &MultilayerAutorouteOptions,
    ) -> Self {
        Self {
            last_configuration: preconfiguration,
            maybe_last_planar_status: None,
            maybe_best_planar_status: None,
        }
    }
}

impl MakeNextMultilayerAutorouteConfiguration
    for IncrementFailedRatlineLayersMultilayerAutorouteReconfigurer
{
    fn process_planar_result(
        &mut self,
        _autorouter: &Autorouter<impl AccessMesadata>,
        planar_result: Result<PlanarAutorouteConfigurationStatus, AutorouterError>,
    ) {
        let Ok(planar_status) = planar_result else {
            return;
        };

        self.maybe_last_planar_status = Some(planar_status.clone());

        if self
            .maybe_best_planar_status
            .as_ref()
            .is_none_or(|status| status.costs.lengths.len() < planar_status.costs.lengths.len())
        {
            self.maybe_best_planar_status = Some(planar_status.clone());
        }
    }

    fn next_configuration(
        &mut self,
        autorouter: &Autorouter<impl AccessMesadata>,
    ) -> Option<MultilayerAutorouteConfiguration> {
        let mut new_anterouter_plan = self.last_configuration.plan.clone();

        let Some(ref last_planar_status) = self.maybe_last_planar_status else {
            return None;
        };

        if let Some(ref best_planar_status) = self.maybe_best_planar_status {
            for ratline_index in best_planar_status.costs.lengths.len()
                ..last_planar_status.configuration.ratlines.len()
            {
                *new_anterouter_plan
                    .layer_map
                    .get_mut(&last_planar_status.configuration.ratlines[ratline_index])
                    .unwrap() += 1;
                *new_anterouter_plan
                    .layer_map
                    .get_mut(&last_planar_status.configuration.ratlines[ratline_index])
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

        Some(self.last_configuration.clone())
    }
}
