use std::ops::ControlFlow;

use spade::InsertionError;
use topola::{
    autorouter::{
        execution::{Command, ExecutionStepper},
        history::History,
        invoker::{Invoker, InvokerError},
        Autorouter,
    },
    board::{mesadata::AccessMesadata, Board},
    stepper::{Abort, Step},
};

use crate::{
    activity::{ActivityContext, ActivityError, ActivityStatus, ActivityStepperWithStatus},
    interaction::InteractionContext,
};

pub struct Interactor<M: AccessMesadata> {
    invoker: Invoker<M>,
    activity: Option<ActivityStepperWithStatus>,
}

impl<M: AccessMesadata> Interactor<M> {
    pub fn new(board: Board<M>) -> Result<Self, InsertionError> {
        Ok(Self {
            invoker: Invoker::new(Autorouter::new(board)?),
            activity: None,
        })
    }

    pub fn execute(&mut self, command: Command) -> Result<(), InvokerError> {
        self.invoker.execute(command)
    }

    pub fn schedule(&mut self, command: Command) -> Result<(), InvokerError> {
        self.activity = Some(ActivityStepperWithStatus::new_execution(
            self.invoker.execute_stepper(command)?,
        ));
        Ok(())
    }

    pub fn undo(&mut self) -> Result<(), InvokerError> {
        self.invoker.undo()
    }

    pub fn redo(&mut self) -> Result<(), InvokerError> {
        self.invoker.redo()
    }

    pub fn abort(&mut self) {
        if let Some(ref mut activity) = self.activity {
            activity.abort(&mut ActivityContext {
                interaction: InteractionContext {},
                invoker: &mut self.invoker,
            });
        }
    }

    pub fn replay(&mut self, history: History) {
        self.invoker.replay(history);
    }

    pub fn update(&mut self) -> ControlFlow<Result<(), ActivityError>> {
        if let Some(ref mut activity) = self.activity {
            return match activity.step(&mut ActivityContext {
                interaction: InteractionContext {},
                invoker: &mut self.invoker,
            }) {
                Ok(ActivityStatus::Running) => ControlFlow::Continue(()),
                Ok(ActivityStatus::Finished(..)) => ControlFlow::Break(Ok(())),
                Err(err) => {
                    self.activity = None;
                    ControlFlow::Break(Err(err))
                }
            };
        }
        ControlFlow::Break(Ok(()))
    }

    pub fn invoker(&self) -> &Invoker<M> {
        &self.invoker
    }

    pub fn maybe_activity(&self) -> &Option<ActivityStepperWithStatus> {
        &self.activity
    }
}
