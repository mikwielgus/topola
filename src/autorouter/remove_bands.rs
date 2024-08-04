use crate::{
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    router::{navmesh::Navmesh, trace::Trace},
};

use super::{
    invoker::{GetGhosts, GetMaybeNavmesh, GetMaybeTrace, GetObstacles},
    selection::BandSelection,
    Autorouter, AutorouterError,
};

#[derive(Debug)]
pub struct RemoveBands {
    selection: BandSelection,
    done: bool,
}

impl RemoveBands {
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

impl GetMaybeNavmesh for RemoveBands {
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        None
    }
}

impl GetMaybeTrace for RemoveBands {
    fn maybe_trace(&self) -> Option<&Trace> {
        None
    }
}

impl GetGhosts for RemoveBands {
    fn ghosts(&self) -> &[PrimitiveShape] {
        &[]
    }
}

impl GetObstacles for RemoveBands {
    fn obstacles(&self) -> &[PrimitiveIndex] {
        &[]
    }
}
