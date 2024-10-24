//! Provides functionality to remove bands from the layout.

use crate::{
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
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
    ) -> Result<(), AutorouterError> {
        if !self.done {
            self.done = true;

            for selector in self.selection.selectors() {
                let band = autorouter.board.bandname_band(&selector.band).unwrap().0;
                autorouter.board.layout_mut().remove_band(band);
            }
            Ok(())
        } else {
            Ok(())
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
