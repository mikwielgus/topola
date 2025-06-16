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
/// Steppers always progress linearly, that is, it is presumed that the context
/// does not change between calls in a way that can affect the stepper's future
/// states. Advanceable data structures designed for uses where the future state
/// intentionally *may* change from the information supplied as arguments are
/// not considered steppers. An example of such an advanceable non-stepper is
/// the [`Navcord`](crate::router::navcord::Navcord) struct, as it does not progress
/// linearly because it branches out by on each call taking in a
/// changeable `to` argument that affects the future states.
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
pub trait StepBack<C, S, E> {
    /// Retreat the stepper's state by one step.
    fn step_back(&mut self, context: &mut C) -> Result<S, E>;
}

/// Steppers that may be aborted implement this trait.
///
/// Aborting a stepper puts it in a state where stepping or stepping back always
/// fails.
pub trait Abort<C> {
    /// Abort the stepper.
    fn abort(&mut self, context: &mut C);
}

/// Steppers that may receive discrete events and act on them, implement this trait.
pub trait OnEvent<Ctx, Event> {
    type Output;

    fn on_event(&mut self, context: &mut Ctx, event: Event) -> Self::Output;
}
