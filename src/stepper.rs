// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use core::ops::ControlFlow;
use std::{collections::VecDeque, time::Instant};

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
    reference: V,
    subscale: S,
}

impl<V, S> LinearScale<V, S> {
    pub fn new(value: V, reference: V, subscale: S) -> Self {
        Self {
            value,
            reference,
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

pub trait GetMaybeReconfigurationTriggerProgress {
    type Subscale;

    fn reconfiguration_trigger_progress(&self) -> Option<LinearScale<f64, Self::Subscale>>;
}

#[derive(Clone, Debug, Getters)]
pub struct SmaRateReconfigurationTrigger {
    #[getter(skip)]
    sample_buffer: VecDeque<f64>,
    #[getter(skip)]
    last_instant: Instant,
    #[getter(skip)]
    last_value: f64,
    maybe_sma_rate_per_sec: Option<f64>,
    #[getter(skip)]
    sample_buffer_size: usize,
    #[getter(skip)]
    sampling_interval_secs: f64,
    min_sma_rate_per_sec: f64,
}

impl SmaRateReconfigurationTrigger {
    pub fn new(
        sample_buffer_size: usize,
        sampling_interval_secs: f64,
        min_sma_rate_per_sec: f64,
    ) -> Self {
        Self {
            sample_buffer: VecDeque::new(),
            last_instant: Instant::now(),
            last_value: 0.0,
            maybe_sma_rate_per_sec: None,
            sample_buffer_size,
            sampling_interval_secs,
            min_sma_rate_per_sec,
        }
    }

    pub fn update(&mut self, value: f64) -> bool {
        let elapsed = self.last_instant.elapsed();
        let delta = value - self.last_value;

        if elapsed.as_secs_f64() >= self.sampling_interval_secs {
            let count = (elapsed.as_secs_f64() / self.sampling_interval_secs) as usize;
            let mut total_pushed = 0.0;
            let mut total_popped = 0.0;

            for _ in 0..count {
                let pushed = delta.max(0.0) / count as f64;
                self.sample_buffer.push_back(delta.max(0.0) / count as f64);
                total_pushed += pushed;

                if self.sample_buffer.len() > self.sample_buffer_size {
                    total_popped += self.sample_buffer.pop_front().unwrap_or_default();
                }
            }

            if let Some(sma_rate_per_sec) = self.maybe_sma_rate_per_sec {
                self.maybe_sma_rate_per_sec = Some(
                    sma_rate_per_sec
                        + (total_pushed - total_popped) / self.sample_buffer_size as f64,
                )
            } else if self.sample_buffer.len() >= self.sample_buffer_size {
                self.maybe_sma_rate_per_sec =
                    Some(self.sample_buffer.iter().sum::<f64>() / self.sample_buffer_size as f64)
            }

            self.last_instant = Instant::now();
            self.last_value = value;
        }

        if let Some(sma_rate_per_sec) = self.maybe_sma_rate_per_sec {
            sma_rate_per_sec >= self.min_sma_rate_per_sec
        } else {
            true
        }
    }
}
