use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::autorouter::invoker::Command;

#[derive(Error, Debug, Clone)]
pub enum HistoryError {
    #[error("no previous command")]
    NoPreviousCommand,
    #[error("no next command")]
    NoNextCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct History {
    done: Vec<Command>,
    undone: Vec<Command>,
}

impl History {
    pub fn new() -> Self {
        Self {
            done: vec![],
            undone: vec![],
        }
    }

    pub fn destructure(self) -> (Vec<Command>, Vec<Command>) {
        (self.done, self.undone)
    }

    pub fn do_(&mut self, command: Command) {
        self.done.push(command);
    }

    pub fn undo(&mut self) -> Result<(), HistoryError> {
        let Some(command) = self.done.pop() else {
            return Err(HistoryError::NoPreviousCommand);
        };

        self.undone.push(command);
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), HistoryError> {
        let Some(command) = self.undone.pop() else {
            return Err(HistoryError::NoNextCommand);
        };

        self.done.push(command);
        Ok(())
    }

    pub fn set_undone(&mut self, iter: impl IntoIterator<Item = Command>) {
        self.undone = Vec::from_iter(iter);
    }

    pub fn last_done(&self) -> Result<&Command, HistoryError> {
        self.done.last().ok_or(HistoryError::NoPreviousCommand)
    }

    pub fn last_undone(&self) -> Result<&Command, HistoryError> {
        self.undone.last().ok_or(HistoryError::NoNextCommand)
    }

    pub fn done(&self) -> &[Command] {
        &self.done
    }

    pub fn undone(&self) -> &[Command] {
        &self.undone
    }
}
