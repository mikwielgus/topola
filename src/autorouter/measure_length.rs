use crate::{
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::{primitive::PrimitiveShape, shape::MeasureLength as MeasureLengthTrait},
    graph::MakeRef,
    router::{navmesh::Navmesh, trace::Trace},
};

use super::{
    invoker::{GetGhosts, GetMaybeNavmesh, GetMaybeTrace, GetObstacles},
    selection::BandSelection,
    Autorouter, AutorouterError,
};

pub struct MeasureLength {
    selection: BandSelection,
    maybe_length: Option<f64>,
}

impl MeasureLength {
    pub fn new(selection: &BandSelection) -> Result<Self, AutorouterError> {
        Ok(Self {
            selection: selection.clone(),
            maybe_length: None,
        })
    }

    pub fn doit(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
    ) -> Result<f64, AutorouterError> {
        let length = if let Some(length) = self.maybe_length {
            length
        } else {
            let mut length = 0.0;

            for selector in self.selection.selectors() {
                let band = autorouter
                    .board
                    .bandname_band(selector.band.clone())
                    .unwrap()
                    .0;
                length += band.ref_(autorouter.board.layout().drawing()).length();
            }

            self.maybe_length = Some(length);
            length
        };

        Ok(length)
    }
}

impl GetMaybeNavmesh for MeasureLength {
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        None
    }
}

impl GetMaybeTrace for MeasureLength {
    fn maybe_trace(&self) -> Option<&Trace> {
        None
    }
}

impl GetGhosts for MeasureLength {
    fn ghosts(&self) -> &[PrimitiveShape] {
        &[]
    }
}

impl GetObstacles for MeasureLength {
    fn obstacles(&self) -> &[PrimitiveIndex] {
        &[]
    }
}
