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
}

impl PlaceVia {
    pub fn new(weight: ViaWeight) -> Result<Self, AutorouterError> {
        Ok(Self { weight })
    }

    pub fn doit(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
    ) -> Result<(), AutorouterError> {
        autorouter.place_via(self.weight)
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
