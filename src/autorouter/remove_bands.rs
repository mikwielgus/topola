//! Provides functionality to remove bands from the layout in an
//! autorouting context. It defines a struct that interacts with the autorouter
//! to remove selected bands, and implements necessary traits for working
//! with navigation meshes, traces, and obstacles.

use crate::{
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    router::{navmesh::Navmesh, trace::TraceStepper},
};

use super::{
    invoker::{GetGhosts, GetMaybeNavmesh, GetMaybeTrace, GetObstacles},
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
                let band = autorouter
                    .board
                    .bandname_band(selector.band.clone())
                    .unwrap()
                    .0;
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

impl GetMaybeTrace for RemoveBandsExecutionStepper {
    fn maybe_trace(&self) -> Option<&TraceStepper> {
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
