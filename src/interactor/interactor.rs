use std::ops::ControlFlow;

use spade::InsertionError;

use crate::{
    autorouter::{
        execution::Command,
        history::History,
        invoker::{Invoker, InvokerError},
        Autorouter,
    },
    board::{mesadata::AccessMesadata, Board},
    interactor::activity::{
        ActivityContext, ActivityError, ActivityStepperWithStatus, InteractiveInput,
    },
    stepper::{Abort, Step},
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
            activity.abort(&mut ActivityContext::<M> {
                interactive_input: &InteractiveInput {
                    pointer_pos: [0.0, 0.0].into(),
                    dt: 0.0,
                },
                invoker: &mut self.invoker,
            });
        }
    }

    pub fn replay(&mut self, history: History) {
        self.invoker.replay(history);
    }

    pub fn update(
        &mut self,
        interactive_input: &InteractiveInput,
    ) -> ControlFlow<Result<(), ActivityError>> {
        if let Some(ref mut activity) = self.activity {
            return match activity.step(&mut ActivityContext {
                interactive_input,
                invoker: &mut self.invoker,
            }) {
                Ok(ControlFlow::Continue(())) => ControlFlow::Continue(()),
                Ok(ControlFlow::Break(msg)) => ControlFlow::Break(Ok(())),
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
