// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, time::SystemTime};

use specctra_core::mesadata::AccessMesadata;

use crate::autorouter::{
    multilayer_autoroute::{MultilayerAutorouteConfiguration, MultilayerAutorouteOptions},
    planar_preconfigurer::PlanarAutoroutePreconfigurerInput,
    Autorouter,
};

pub struct MultilayerReconfigurer {
    preconfiguration: MultilayerAutorouteConfiguration,
}

impl MultilayerReconfigurer {
    pub fn new(
        _autorouter: &Autorouter<impl AccessMesadata>,
        preconfiguration: MultilayerAutorouteConfiguration,
        _options: &MultilayerAutorouteOptions,
    ) -> Self {
        Self { preconfiguration }
    }

    pub fn next_configuration(
        &mut self,
        _autorouter: &Autorouter<impl AccessMesadata>,
    ) -> Option<MultilayerAutorouteConfiguration> {
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

        Some(MultilayerAutorouteConfiguration {
            plan: new_anterouter_plan,
            planar: PlanarAutoroutePreconfigurerInput {
                ratlines: self.preconfiguration.planar.ratlines.clone(),
                terminating_dot_map: BTreeMap::new(),
            },
        })
    }

    fn crude_random_bit() -> usize {
        let duration_since_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let timestamp_nanos = duration_since_epoch.as_nanos();
        (timestamp_nanos % 2) as usize
    }
}
