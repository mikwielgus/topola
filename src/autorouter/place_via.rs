use crate::{
    autorouter::{
        invoker::{GetMaybeNavmesh, GetMaybeTrace},
        Autorouter, AutorouterError,
    },
    board::mesadata::AccessMesadata,
    layout::via::ViaWeight,
    router::{navmesh::Navmesh, trace::Trace},
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
