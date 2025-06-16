// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::ops::ControlFlow;

use spade::InsertionError;

use crate::{
    autorouter::{
        execution::Command,
        history::History,
        invoker::{Invoker, InvokerError},
        Autorouter,
    },
    board::{AccessMesadata, Board},
    interactor::{
        activity::{
            ActivityContext, ActivityError, ActivityStepperWithStatus, InteractiveEvent,
            InteractiveInput,
        },
        interaction::InteractionStepper,
    },
    stepper::{Abort, OnEvent, Step},
};

/// Structure that manages the invoker and activities
pub struct Interactor<M> {
    invoker: Invoker<M>,
    activity: Option<ActivityStepperWithStatus>,
}

impl<M: AccessMesadata> Interactor<M> {
    /// Create a new instance of Interactor with the given Board instance
    pub fn new(board: Board<M>) -> Result<Self, InsertionError> {
        Ok(Self {
            invoker: Invoker::new(Autorouter::new(board)?),
            activity: None,
        })
    }

    /// Execute a command
    pub fn execute(&mut self, command: Command) -> Result<(), InvokerError> {
        self.invoker.execute(command)
    }

    /// Start executing an activity
    pub fn schedule(&mut self, command: Command) -> Result<(), InvokerError> {
        self.activity = Some(ActivityStepperWithStatus::new_execution(
            self.invoker.execute_stepper(command)?,
        ));
        Ok(())
    }

    /// Start an interaction activity
    pub fn interact(&mut self, interaction: InteractionStepper) {
        self.activity = Some(ActivityStepperWithStatus::new_interaction(interaction));
    }

    /// Undo last command
    pub fn undo(&mut self) -> Result<(), InvokerError> {
        self.invoker.undo()
    }

    /// Redo last command
    pub fn redo(&mut self) -> Result<(), InvokerError> {
        self.invoker.redo()
    }

    /// Abort the currently running execution or activity
    pub fn abort(&mut self) {
        if let Some(ref mut activity) = self.activity.take() {
            activity.abort(&mut self.invoker);
        }
    }

    /// Replay last command
    pub fn replay(&mut self, history: History) {
        self.invoker.replay(history);
    }

    /// Update the currently running execution or activity
    pub fn update(
        &mut self,
        interactive_input: &InteractiveInput,
        interactive_event: Option<InteractiveEvent>,
    ) -> ControlFlow<Result<(), ActivityError>> {
        if let Some(ref mut activity) = self.activity {
            if let Some(event) = interactive_event {
                match activity.on_event(
                    &mut ActivityContext {
                        interactive_input,
                        invoker: &mut self.invoker,
                    },
                    event,
                ) {
                    Ok(()) => ControlFlow::Continue(()),
                    Err(err) => {
                        self.activity = None;
                        ControlFlow::Break(Err(err.into()))
                    }
                }
            } else {
                match activity.step(&mut ActivityContext {
                    interactive_input,
                    invoker: &mut self.invoker,
                }) {
                    Ok(ControlFlow::Continue(())) => ControlFlow::Continue(()),
                    Ok(ControlFlow::Break(_msg)) => {
                        self.activity = None;
                        ControlFlow::Break(Ok(()))
                    }
                    Err(err) => {
                        self.activity = None;
                        ControlFlow::Break(Err(err))
                    }
                }
            }
        } else {
            ControlFlow::Break(Ok(()))
        }
    }

    /// Returns the invoker
    pub fn invoker(&self) -> &Invoker<M> {
        &self.invoker
    }

    /// Returns the currently running activity
    pub fn maybe_activity(&self) -> &Option<ActivityStepperWithStatus> {
        &self.activity
    }
}
