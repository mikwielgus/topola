// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

//! Provides functionality to remove bands from the layout.

use crate::{
    board::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    layout::LayoutEdit,
    router::{navcord::NavcordStepper, navmesh::Navmesh},
};

use super::{
    invoker::{GetGhosts, GetMaybeNavcord, GetMaybeNavmesh, GetObstacles},
    selection::BandSelection,
    Autorouter, AutorouterError,
};

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
                let band = autorouter.board.bandname_band(&selector.band).unwrap()[false];
                autorouter.board.layout_mut().remove_band(&mut edit, band);
            }
            Ok(Some(edit))
        } else {
            Ok(None)
        }
    }
}

impl GetMaybeNavmesh for RemoveBandsExecutionStepper {
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        None
    }
}

impl GetMaybeNavcord for RemoveBandsExecutionStepper {
    fn maybe_navcord(&self) -> Option<&NavcordStepper> {
        None
    }
}

impl GetGhosts for RemoveBandsExecutionStepper {
    fn ghosts(&self) -> &[PrimitiveShape] {
        &[]
    }
}

impl GetObstacles for RemoveBandsExecutionStepper {
    fn obstacles(&self) -> &[PrimitiveIndex] {
        &[]
    }
}
