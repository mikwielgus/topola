//! Provides functionality for placing vias in a PCB layout, manages
//! the process of inserting a via with a specified weight and
//! checks if the via has already been placed.

use crate::{
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    layout::via::ViaWeight,
    router::{navcord::NavcordStepper, navmesh::Navmesh},
};

use super::{
    invoker::{GetGhosts, GetMaybeNavcord, GetMaybeNavmesh, GetObstacles},
    Autorouter, AutorouterError,
};

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

impl GetMaybeNavmesh for PlaceViaExecutionStepper {
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        None
    }
}

impl GetMaybeNavcord for PlaceViaExecutionStepper {
    fn maybe_navcord(&self) -> Option<&NavcordStepper> {
        None
    }
}

impl GetGhosts for PlaceViaExecutionStepper {
    fn ghosts(&self) -> &[PrimitiveShape] {
        &[]
    }
}

impl GetObstacles for PlaceViaExecutionStepper {
    fn obstacles(&self) -> &[PrimitiveIndex] {
        &[]
    }
}
