// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

//! Provides functionality for placing vias in a PCB layout, manages
//! the process of inserting a via with a specified weight and
//! checks if the via has already been placed.

use crate::{
    board::{
        edit::{BoardDataEdit, BoardEdit},
        AccessMesadata,
    },
    layout::{via::ViaWeight, LayoutEdit},
    stepper::EstimateLinearProgress,
};

use super::{invoker::GetDebugOverlayData, Autorouter, AutorouterError};

#[derive(Debug)]
pub struct PlaceViaExecutionStepper {
    weight: ViaWeight,
    done: bool,
}

impl PlaceViaExecutionStepper {
    pub fn new(weight: ViaWeight) -> Result<Self, AutorouterError> {
        Ok(Self {
            weight,
            done: false,
        })
    }

    pub fn doit(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
    ) -> Result<Option<BoardEdit>, AutorouterError> {
        if !self.done {
            self.done = true;

            let mut layout_edit = LayoutEdit::new();
            autorouter
                .board
                .layout_mut()
                .add_via(&mut layout_edit, self.weight)?;
            Ok(Some(BoardEdit::new_from_edits(
                BoardDataEdit::new(),
                layout_edit,
            )))
        } else {
            Ok(None)
        }
    }
}

impl EstimateLinearProgress for PlaceViaExecutionStepper {
    type Value = f64;
}
impl GetDebugOverlayData for PlaceViaExecutionStepper {}
