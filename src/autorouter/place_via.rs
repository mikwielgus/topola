// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

//! Provides functionality for placing vias in a PCB layout, manages
//! the process of inserting a via with a specified weight and
//! checks if the via has already been placed.

use crate::{
    board::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    layout::{via::ViaWeight, LayoutEdit},
    router::{
        astar::AstarStepper,
        navcord::Navcord,
        navmesh::{Navmesh, NavvertexIndex},
    },
};

use super::{
    invoker::{
        GetGhosts, GetMaybeAstarStepper, GetMaybeNavcord, GetNavmeshDebugTexts, GetObstacles,
    },
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
    ) -> Result<Option<LayoutEdit>, AutorouterError> {
        if !self.done {
            self.done = true;

            let mut edit = LayoutEdit::new();
            autorouter
                .board
                .layout_mut()
                .add_via(&mut edit, self.weight)?;
            Ok(Some(edit))
        } else {
            Ok(None)
        }
    }
}

impl GetMaybeAstarStepper for PlaceViaExecutionStepper {
    fn maybe_astar(&self) -> Option<&AstarStepper<Navmesh, f64>> {
        None
    }
}

impl GetMaybeNavcord for PlaceViaExecutionStepper {
    fn maybe_navcord(&self) -> Option<&Navcord> {
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

impl GetNavmeshDebugTexts for PlaceViaExecutionStepper {
    fn navvertex_debug_text(&self, _navvertex: NavvertexIndex) -> Option<&str> {
        None
    }

    fn navedge_debug_text(&self, _navedge: (NavvertexIndex, NavvertexIndex)) -> Option<&str> {
        None
    }
}
