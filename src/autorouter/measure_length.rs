// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

//! Provides functionality for measuring the total length of selected
//! bands in a PCB layout. It interacts with the autorouter to calculate and return
//! the length of specified band selections.

use crate::{
    board::AccessMesadata, geometry::shape::MeasureLength as MeasureLengthTrait, graph::MakeRef,
};

use super::{
    invoker::{
        GetGhosts, GetMaybeAstarStepper, GetMaybeNavcord, GetNavmeshDebugTexts, GetObstacles,
    },
    selection::BandSelection,
    Autorouter, AutorouterError,
};

pub struct MeasureLengthExecutionStepper {
    selection: BandSelection,
    maybe_length: Option<f64>,
}

impl MeasureLengthExecutionStepper {
    pub fn new(selection: &BandSelection) -> Result<Self, AutorouterError> {
        Ok(Self {
            selection: selection.clone(),
            maybe_length: None,
        })
    }

    pub fn doit(
        &mut self,
        autorouter: &Autorouter<impl AccessMesadata>,
    ) -> Result<f64, AutorouterError> {
        let length = if let Some(length) = self.maybe_length {
            length
        } else {
            let mut length = 0.0;

            for selector in self.selection.selectors() {
                let band = autorouter.board.bandname_band(&selector.band).unwrap()[false];
                length += band.ref_(autorouter.board.layout().drawing()).length();
            }

            self.maybe_length = Some(length);
            length
        };

        Ok(length)
    }
}

impl GetMaybeAstarStepper for MeasureLengthExecutionStepper {}
impl GetMaybeNavcord for MeasureLengthExecutionStepper {}
impl GetGhosts for MeasureLengthExecutionStepper {}
impl GetObstacles for MeasureLengthExecutionStepper {}
impl GetNavmeshDebugTexts for MeasureLengthExecutionStepper {}
