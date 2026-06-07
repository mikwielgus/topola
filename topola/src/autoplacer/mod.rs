// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

pub mod interactors;

use std::ops::ControlFlow;

use rand::RngExt;
use rand_distr::{Distribution, Normal};
use undoredo::{FlushDelta, ResetDelta};

use crate::{
    board::Board,
    layout::{Layout, compounds::ComponentId},
    orientation::Orientation,
    selections::ComponentSelection,
    vector::Vector2,
};

pub struct AutoplacerSchedule {
    pub initial_temperature: f64,
    pub temperature_common_ratio: f64,
    pub initial_std_dev: f64,
    pub std_dev_common_ratio: f64,
    pub max_steps: u64,
}

#[derive(Clone, Copy)]
pub struct AutoplacerStepParams {
    temperature: f64,
    std_dev: f64,
}

pub struct Autoplacer {
    components: Vec<ComponentId>,
    schedule: AutoplacerSchedule,
    step_counter: u64,
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
        }
    }

    pub fn step(&mut self, board: &mut Board) -> ControlFlow<()> {
        crate::profile_function!();

        if self.step_counter < self.schedule.max_steps {
            let control_flow = self.step_with_params(
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
            );

            self.step_counter += 1;
            control_flow
        } else {
            ControlFlow::Break(())
        }
    }

    // TODO.
    /*pub fn reject(&mut self) {

    }*/

    fn step_with_params(
        &mut self,
        board: &mut Board,
        params: AutoplacerStepParams,
    ) -> ControlFlow<()> {
        crate::profile_function!();

        for i in 0..self.components.len() {
            let component = self.components[i];
            self.step_component(board, component, params);
        }

        ControlFlow::Continue(())
    }

    fn step_component(
        &mut self,
        board: &mut Board,
        component: ComponentId,
        params: AutoplacerStepParams,
    ) {
        crate::profile_function!();

        let last_cost = self.component_cost(board, component, params);
        let translation = self.sample_move(params);
        board.move_resolved_components_by(&[component], translation);
        let new_cost = self.component_cost(board, component, params);
        let delta_cost = new_cost - last_cost;

        if delta_cost < 0.0
            || rand::rng().random::<f64>() < f64::exp(-delta_cost / params.temperature)
        {
            self.accept_move(board);
        } else {
            self.reject_move(board);
        }
    }

    fn sample_move(&self, params: AutoplacerStepParams) -> Vector2<i64> {
        crate::profile_function!();
        let dx_gaussian = Normal::new(0.0, params.std_dev).unwrap();
        let dy_gaussian = Normal::new(0.0, params.std_dev).unwrap();
        Vector2::new(
            dx_gaussian.sample(&mut rand::rng()) as i64,
            dy_gaussian.sample(&mut rand::rng()) as i64,
        )
    }

    fn accept_move(&mut self, board: &mut Board) {
        crate::profile_function!();
        //self.origin_delta = self.origin_delta.clone().merge_delta(board.flush_delta());
        board.flush_delta();
    }

    fn reject_move(&mut self, board: &mut Board) {
        crate::profile_function!();
        board.reset_delta();
    }

    /*fn cost(&self, board: &Board, params: AutoplacerStepParams) -> f64 {
        self.components
            .iter()
            .map(|&component| self.component_cost(board, component, params))
            .sum()
    }*/

    fn component_cost(
        &self,
        board: &Board,
        component: ComponentId,
        _params: AutoplacerStepParams,
    ) -> f64 {
        crate::profile_function!();

        let layout = board.layout();
        let repulsion_cost = self.repulsion_cost(layout, component);
        let attraction_cost = self.attraction_cost(layout, component);
        let retention_cost = self.retention_cost(layout, component);

        repulsion_cost as f64 + attraction_cost + retention_cost as f64
    }

    fn repulsion_cost(&self, layout: &Layout, component: ComponentId) -> i64 {
        crate::profile_function!();
        layout
            .locate_component_repulsions(component, Orientation::Oblique)
            .map(|vector| 1000 * (vector.x.abs() + vector.y.abs()))
            .sum()
    }

    fn attraction_cost(&self, layout: &Layout, component: ComponentId) -> f64 {
        crate::profile_function!();
        layout
            .component_attractions(component)
            .map(|vector| {
                (vector.x.abs().pow(2) as f64 + vector.y.abs().pow(2) as f64)
                    .sqrt()
                    .powf(0.5)
            })
            .sum()
    }

    fn retention_cost(&self, layout: &Layout, component: ComponentId) -> i64 {
        crate::profile_function!();
        layout
            .component_retentions(component)
            .map(|vector| 1000 * (vector.x.abs() + vector.y.abs()))
            .sum()
    }

    /*fn step_component_with_params(
        &mut self,
        component: ComponentId,
        params: AutoplacerStepParams,
    ) -> bool {
        //
    }*/
}
