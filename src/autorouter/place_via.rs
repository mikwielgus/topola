use crate::{
    autorouter::{invoker::GetMaybeNavmesh, Autorouter, AutorouterError},
    board::mesadata::MesadataTrait,
    layout::via::ViaWeight,
    router::navmesh::Navmesh,
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
        autorouter: &mut Autorouter<impl MesadataTrait>,
    ) -> Result<(), AutorouterError> {
        autorouter.place_via(self.weight)
    }
}

impl GetMaybeNavmesh for PlaceVia {
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        None
    }
}
