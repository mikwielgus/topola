use crate::{
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::{primitive::PrimitiveShape, shape::MeasureLength as MeasureLengthTrait},
    graph::MakeRef,
    router::{navmesh::Navmesh, trace::TraceStepper},
};

use super::{
    invoker::{GetGhosts, GetMaybeNavmesh, GetMaybeTrace, GetObstacles},
    selection::BandSelection,
    Autorouter, AutorouterError,
};

pub struct MeasureLengthCommandStepper {
    selection: BandSelection,
    maybe_length: Option<f64>,
}

impl MeasureLengthCommandStepper {
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

impl GetMaybeNavmesh for MeasureLengthCommandStepper {
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        None
    }
}

impl GetMaybeTrace for MeasureLengthCommandStepper {
    fn maybe_trace(&self) -> Option<&TraceStepper> {
        None
    }
}

impl GetGhosts for MeasureLengthCommandStepper {
    fn ghosts(&self) -> &[PrimitiveShape] {
        &[]
    }
}

impl GetObstacles for MeasureLengthCommandStepper {
    fn obstacles(&self) -> &[PrimitiveIndex] {
        &[]
    }
}
