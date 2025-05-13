// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

//! Provides functionality for measuring the total length of selected
//! bands in a PCB layout. It interacts with the autorouter to calculate and return
//! the length of specified band selections.

use crate::{
    board::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::{primitive::PrimitiveShape, shape::MeasureLength as MeasureLengthTrait},
    graph::MakeRef,
    router::{
        astar::AstarStepper,
        navcord::Navcord,
        navmesh::{Navmesh, NavvertexIndex},
    },
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

impl GetMaybeAstarStepper for MeasureLengthExecutionStepper {
    fn maybe_astar(&self) -> Option<&AstarStepper<Navmesh, f64>> {
        None
    }
}

impl GetMaybeNavcord for MeasureLengthExecutionStepper {
    fn maybe_navcord(&self) -> Option<&Navcord> {
        None
    }
}

impl GetGhosts for MeasureLengthExecutionStepper {
    fn ghosts(&self) -> &[PrimitiveShape] {
        &[]
    }
}

impl GetObstacles for MeasureLengthExecutionStepper {
    fn obstacles(&self) -> &[PrimitiveIndex] {
        &[]
    }
}

impl GetNavmeshDebugTexts for MeasureLengthExecutionStepper {
    fn navvertex_debug_text(&self, _navvertex: NavvertexIndex) -> Option<&str> {
        None
    }

    fn navedge_debug_text(&self, _navedge: (NavvertexIndex, NavvertexIndex)) -> Option<&str> {
        None
    }
}
