// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::time::SystemTime;

use specctra_core::mesadata::AccessMesadata;

use crate::autorouter::{
    anterouter::AnterouterPlan, multilayer_autoroute::MultilayerAutorouteOptions, planner::Planner,
    ratline::RatlineUid, Autorouter,
};

pub struct MultilayerReconfigurer {
    original_ratlines: Vec<RatlineUid>,
}

impl MultilayerReconfigurer {
    pub fn new(
        autorouter: &Autorouter<impl AccessMesadata>,
        ratlines: Vec<RatlineUid>,
        options: &MultilayerAutorouteOptions,
    ) -> Self {
        Self {
            original_ratlines: ratlines,
        }
    }

    pub fn next_configuration(
        &mut self,
        autorouter: &Autorouter<impl AccessMesadata>,
    ) -> Option<AnterouterPlan> {
        let planner = Planner::new_from_layer_map(
            autorouter,
            &self.original_ratlines,
            self.original_ratlines
                .iter()
                .enumerate()
                .map(|(_, ratline)| (*ratline, Self::crude_random_bit()))
                .collect(),
        );

        Some(planner.plan().clone())
    }

    fn crude_random_bit() -> usize {
        let duration_since_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let timestamp_nanos = duration_since_epoch.as_nanos();
        (timestamp_nanos % 2) as usize
    }
}
