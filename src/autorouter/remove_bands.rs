// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

//! Provides functionality to remove bands from the layout.

use crate::{board::AccessMesadata, layout::LayoutEdit};

use super::{invoker::GetDebugOverlayData, selection::BandSelection, Autorouter, AutorouterError};

#[derive(Debug)]
pub struct RemoveBandsExecutionStepper {
    selection: BandSelection,
    done: bool,
}

impl RemoveBandsExecutionStepper {
    pub fn new(selection: &BandSelection) -> Result<Self, AutorouterError> {
        Ok(Self {
            selection: selection.clone(),
            done: false,
        })
    }

    pub fn doit(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
    ) -> Result<Option<LayoutEdit>, AutorouterError> {
        if !self.done {
            self.done = true;

            let mut edit = LayoutEdit::new();
            for selector in self.selection.selectors() {
                let band = *autorouter.board.bandname_band(&selector.band).unwrap();
                autorouter
                    .board
                    .remove_band_by_id(&mut edit, band)
                    .map_err(|_| AutorouterError::CouldNotRemoveBand(band[false]))?;
            }
            Ok(Some(edit))
        } else {
            Ok(None)
        }
    }
}

impl GetDebugOverlayData for RemoveBandsExecutionStepper {}
