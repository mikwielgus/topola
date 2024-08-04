use crate::{
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    layout::via::ViaWeight,
    router::{navmesh::Navmesh, trace::Trace},
};

use super::{
    invoker::{GetGhosts, GetMaybeNavmesh, GetMaybeTrace, GetObstacles},
    Autorouter, AutorouterError,
};

#[derive(Debug)]
pub struct PlaceVia {
    weight: ViaWeight,
    done: bool,
}

impl PlaceVia {
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

impl GetMaybeNavmesh for PlaceVia {
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        None
    }
}

impl GetMaybeTrace for PlaceVia {
    fn maybe_trace(&self) -> Option<&Trace> {
        None
    }
}

impl GetGhosts for PlaceVia {
    fn ghosts(&self) -> &[PrimitiveShape] {
        &[]
    }
}

impl GetObstacles for PlaceVia {
    fn obstacles(&self) -> &[PrimitiveIndex] {
        &[]
    }
}
