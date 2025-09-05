// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use core::ops::ControlFlow;

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
pub trait Permutate<Ctx> {
    type Index;
    type Output;

    fn permutate(&mut self, context: &mut Ctx, ordering: Vec<Self::Index>) -> Self::Output;
}

/// Steppers that can receive discrete events and act on them implement this
/// trait.
// XXX: Doesn't this violate the rule that stepper's future states are
// determined by its initial state?
pub trait OnEvent<Ctx, Event> {
    type Output;

    fn on_event(&mut self, context: &mut Ctx, event: Event) -> Self::Output;
}

/// Some steppers report estimates of how far they are from completion.
pub trait EstimateProgress {
    type Value: Default;

    fn estimate_progress_value(&self) -> Self::Value {
        Self::Value::default()
    }

    fn estimate_progress_maximum(&self) -> Self::Value {
        Self::Value::default()
    }
}
