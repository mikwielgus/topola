use crate::{
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    layout::via::ViaWeight,
    router::{navmesh::Navmesh, trace::TraceStepper},
};

use super::{
    invoker::{GetGhosts, GetMaybeNavmesh, GetMaybeTrace, GetObstacles},
    Autorouter, AutorouterError,
};

#[derive(Debug)]
pub struct PlaceViaCommandStepper {
    weight: ViaWeight,
    done: bool,
}

impl PlaceViaCommandStepper {
    pub fn new(weight: ViaWeight) -> Result<Self, AutorouterError> {
        Ok(Self {
            weight,
            done: false,
        })
    }

    pub fn doit(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
    ) -> Result<(), AutorouterError> {
        if !self.done {
            self.done = true;
            autorouter.board.layout_mut().add_via(self.weight)?;
            Ok(())
        } else {
            Ok(())
        }
    }
}

impl GetMaybeNavmesh for PlaceViaCommandStepper {
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        None
    }
}

impl GetMaybeTrace for PlaceViaCommandStepper {
    fn maybe_trace(&self) -> Option<&TraceStepper> {
        None
    }
}

impl GetGhosts for PlaceViaCommandStepper {
    fn ghosts(&self) -> &[PrimitiveShape] {
        &[]
    }
}

impl GetObstacles for PlaceViaCommandStepper {
    fn obstacles(&self) -> &[PrimitiveIndex] {
        &[]
    }
}
