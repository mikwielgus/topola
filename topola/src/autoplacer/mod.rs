// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

pub mod interactors;

use rand::RngExt;
use rand_distr::{Distribution, Normal};
use undoredo::{FlushDelta, ResetDelta};

use crate::{
    board::{Board, BoardDelta},
    layout::compounds::ComponentId,
    orientation::Orientation,
    selections::ComponentSelection,
    vector::Vector2,
};

pub struct AutoplacerSchedule {
    initial_temperature: f64,
    temperature_common_ratio: f64,
    initial_std_dev: f64,
    std_dev_common_ratio: f64,
}

#[derive(Clone, Copy)]
pub struct AutoplacerStepParams {
    temperature: f64,
    std_dev: f64,
}

pub struct Autoplacer {
    components: Vec<ComponentId>,
    schedule: AutoplacerSchedule,
    step_counter: u32,
    origin_delta: BoardDelta, //rng: ThreadRng,
}

impl Autoplacer {
    pub fn new(
        board: &mut Board,
        selection: ComponentSelection,
        schedule: AutoplacerSchedule,
    ) -> Self {
        Self {
            components: board.resolve_components(selection).collect(),
            schedule,
            step_counter: 0,
            origin_delta: board.flush_delta(),
        }
    }

    pub fn step(&mut self, board: &mut Board) -> bool {
        self.step_with_params(
            board,
            AutoplacerStepParams {
                temperature: self.schedule.initial_temperature
                    * self
                        .schedule
                        .temperature_common_ratio
                        .powf(self.step_counter as f64),
                std_dev: self.schedule.initial_std_dev
                    * self
                        .schedule
                        .std_dev_common_ratio
                        .powf(self.step_counter as f64),
            },
        )
    }

    // TODO.
    /*pub fn reject(&mut self) {

    }*/

    fn step_with_params(&mut self, board: &mut Board, params: AutoplacerStepParams) -> bool {
        for &component in self.components.iter() {
            //self.step_component_with_params(component, params);
            let last_cost = self.cost(board, params);

            let dx_gaussian = Normal::new(0.0, params.std_dev).unwrap();
            let dy_gaussian = Normal::new(0.0, params.std_dev).unwrap();

            //let dx = dx_gaussian.sample(&mut self.rng);
            //let dy = dy_gaussian.sample(&mut self.rng);
            let dx = dx_gaussian.sample(&mut rand::rng());
            let dy = dy_gaussian.sample(&mut rand::rng());

            board.move_resolved_components_by(&self.components, Vector2::new(dx as i64, dy as i64));

            let new_cost = self.cost(board, params);
            let delta_cost = new_cost - last_cost;

            if delta_cost <= 0.0
                || f64::exp(-delta_cost / params.temperature) <= rand::rng().random()
            {
                self.origin_delta = self.origin_delta.clone().merge_delta(board.flush_delta());
            } else {
                board.reset_delta();
            }
        }

        true
    }

    fn cost(&self, board: &Board, params: AutoplacerStepParams) -> f64 {
        self.components
            .iter()
            .map(|&component| self.component_cost(board, component, params))
            .sum()
    }

    fn component_cost(
        &self,
        board: &Board,
        component: ComponentId,
        _params: AutoplacerStepParams,
    ) -> f64 {
        let layout = board.layout();

        let repulsion_cost: i64 = layout
            .locate_component_repulsions(component, Orientation::Oblique)
            .map(|vector| vector.x.abs() + vector.y.abs())
            .sum();
        let attraction_cost: f64 = layout
            .component_attractions(component)
            .map(|vector| 1.0 / (1.0 + (vector.x.abs() + vector.y.abs()) as f64))
            .sum();

        repulsion_cost as f64 + attraction_cost
    }

    /*fn step_component_with_params(
        &mut self,
        component: ComponentId,
        params: AutoplacerStepParams,
    ) -> bool {
        //
    }*/
}
