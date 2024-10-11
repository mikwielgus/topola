//! Manages command history operations, allowing for undoing and redoing commands.
//! Handles error scenarios related to command history, maintaining lists of executed
//! and undone commands for easy navigation.

use derive_getters::{Dissolve, Getters};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::autorouter::execution::Command;

#[derive(Error, Debug, Clone)]
pub enum HistoryError {
    #[error("no previous command")]
    NoPreviousCommand,
    #[error("no next command")]
    NoNextCommand,
}

#[derive(Debug, Default, Clone, Getters, Dissolve, Serialize, Deserialize)]
pub struct History {
    done: Vec<Command>,
    undone: Vec<Command>,
}

impl History {
    pub fn new() -> Self {
        Self::default()
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
}
