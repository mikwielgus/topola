use crate::{
    autorouter::{Autorouter, AutorouterError},
    board::mesadata::MesadataTrait,
    layout::via::ViaWeight,
};

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
