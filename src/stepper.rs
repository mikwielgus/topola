// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use core::ops::ControlFlow;
use std::time::Instant;

use derive_getters::Getters;

/// This trait represents a linearly advanceable state whose advancement may
/// break or fail with many different return values, and to which part of
/// the information, called the context, has to be supplied on each call as a
/// mutable reference argument.
///
/// An object that implements this trait is called a "stepper".
///
/// Steppers always progress linearly and their future states are determined by
/// the initial state. It is assumed that the changes in context cannot change
/// the stepper's execution. Advanceable data structures designed for uses where
/// the future state intentionally *may* change from the information supplied
/// after initialization are not considered steppers. An example of such an
/// advanceable non-stepper is the [`Navcord`] (crate::router::navcord::Navcord)
/// struct, as it does not progress linearly because it branches out on each
/// call by taking in a changeable `to` argument that affects the future states.
///
/// Petgraph's counterpart of this trait is its
/// [`petgraph::visit::Walker<Context>`] trait.
pub trait Step<Ctx, B, C = ()> {
    type Error;

    /// Advance the stepper's state by one step.
    fn step(&mut self, context: &mut Ctx) -> Result<ControlFlow<B, C>, Self::Error>;

    /// Advance the stepper step-by-step in a loop until it fails or breaks.
    fn finish(&mut self, context: &mut Ctx) -> Result<B, Self::Error> {
        loop {
            if let ControlFlow::Break(outcome) = self.step(context)? {
                return Ok(outcome);
            }
        }
    }
}

/// Steppers that may be stepped backwards implement this trait.
pub trait StepBack<Ctx, S, E> {
    /// Retreat the stepper's state by one step.
    fn step_back(&mut self, context: &mut Ctx) -> Result<S, E>;
}

/// Steppers that may be aborted implement this trait.
///
/// Aborting a stepper puts it and its context back in its initial state, except
/// that from then on trying to step or step back always fails.
pub trait Abort<Ctx> {
    /// Abort the stepper.
    fn abort(&mut self, context: &mut Ctx);
}

/// Some steppers may be permuted from their initial order.
pub trait Reconfigure<Ctx> {
    type Configuration;
    type Output;

    fn reconfigure(
        &mut self,
        context: &mut Ctx,
        configuration: Self::Configuration,
    ) -> Self::Output;
}

#[derive(Clone, Copy, Debug)]
pub enum ReconfiguratorStatus<Re, Ru> {
    Running(Ru),
    Reconfigured(Re),
}

/// Steppers that can receive discrete events and act on them implement this
/// trait.
// XXX: Doesn't this violate the rule that stepper's future states are
// determined by its initial state?
pub trait OnEvent<Ctx, Event> {
    type Output;

    fn on_event(&mut self, context: &mut Ctx, event: Event) -> Self::Output;
}

#[derive(Clone, Copy, Debug, Getters)]
pub struct LinearScale<V, S = ()> {
    value: V,
    maximum: V,
    subscale: S,
}

impl<V, S> LinearScale<V, S> {
    pub fn new(value: V, reference: V, subscale: S) -> Self {
        Self {
            value,
            maximum: reference,
            subscale,
        }
    }
}

/// Some steppers report estimates of how far they are from completion.
pub trait EstimateProgress {
    type Value;
    type Subscale;

    fn estimate_progress(&self) -> LinearScale<Self::Value, Self::Subscale>;
}

pub trait GetTimeoutProgress {
    type Subscale;

    fn timeout_progress(&self) -> Option<LinearScale<f64, Self::Subscale>>;
}

#[derive(Clone, Debug, Getters)]
pub struct TimeVsProgressAccumulatorTimeout {
    start_instant: Instant,
    #[getter(skip)]
    last_max_value: f64,
    progress_accumulator: f64,
    #[getter(skip)]
    progress_bonus_s: f64,
}

impl TimeVsProgressAccumulatorTimeout {
    pub fn new(initial_timeout_s: f64, progress_bonus_s: f64) -> Self {
        Self {
            start_instant: Instant::now(),
            last_max_value: 0.0,
            progress_accumulator: initial_timeout_s,
            progress_bonus_s,
        }
    }

    pub fn update(&mut self, value: f64) -> bool {
        if value > self.last_max_value {
            self.progress_accumulator += (value - self.last_max_value) * self.progress_bonus_s;
            self.last_max_value = value;
        }

        self.start_instant.elapsed().as_secs_f64() < self.progress_accumulator
    }
}
