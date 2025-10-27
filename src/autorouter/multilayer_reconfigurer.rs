// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, ops::ControlFlow, time::SystemTime};

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
    UniformRandomLayers(UniformRandomLayersMultilayerAutorouteReconfigurer),
}

pub struct UniformRandomLayersMultilayerAutorouteReconfigurer {
    preconfiguration: MultilayerAutorouteConfiguration,
    planar_autoroute_reconfiguration_count: u64,
}

impl UniformRandomLayersMultilayerAutorouteReconfigurer {
    pub fn new(
        _autorouter: &Autorouter<impl AccessMesadata>,
        preconfiguration: MultilayerAutorouteConfiguration,
        _options: &MultilayerAutorouteOptions,
    ) -> Self {
        Self {
            preconfiguration,
            planar_autoroute_reconfiguration_count: 0,
        }
    }

    fn crude_random_bit() -> usize {
        let duration_since_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let timestamp_nanos = duration_since_epoch.as_nanos();
        (timestamp_nanos % 2) as usize
    }
}

impl MakeNextMultilayerAutorouteConfiguration
    for UniformRandomLayersMultilayerAutorouteReconfigurer
{
    fn next_configuration(
        &mut self,
        _autorouter: &Autorouter<impl AccessMesadata>,
        planar_result: Result<PlanarAutorouteConfigurationStatus, AutorouterError>,
    ) -> ControlFlow<Option<MultilayerAutorouteConfiguration>> {
        if self.planar_autoroute_reconfiguration_count < 100 {
            self.planar_autoroute_reconfiguration_count += 1;
            return ControlFlow::Continue(());
        }

        self.planar_autoroute_reconfiguration_count = 0;

        let mut new_anterouter_plan = self.preconfiguration.plan.clone();
        new_anterouter_plan.layer_map = self
            .preconfiguration
            .planar
            .ratlines
            .iter()
            .enumerate()
            //.map(|(i, ratline)| (*ratline, i % 2))
            .map(|(_, ratline)| (*ratline, Self::crude_random_bit()))
            .collect();

        ControlFlow::Break(Some(MultilayerAutorouteConfiguration {
            plan: new_anterouter_plan,
            planar: PlanarAutoroutePreconfigurerInput {
                ratlines: self.preconfiguration.planar.ratlines.clone(),
                terminating_dot_map: BTreeMap::new(),
            },
        }))
    }
}
