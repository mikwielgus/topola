// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use spade::InsertionError;
use specctra_core::mesadata::AccessMesadata;

use crate::{autorouter::ratsnest::Ratsnest, board::Board};

pub struct Ratsnests(Box<[Ratsnest]>);

impl Ratsnests {
    pub fn new(board: &Board<impl AccessMesadata>) -> Result<Self, InsertionError> {
        Ok(Self(Box::new([Ratsnest::new(board)?])))
    }

    pub fn on_principal_layer(&self, principal_layer: usize) -> &Ratsnest {
        &self.0[principal_layer]
    }

    pub fn on_principal_layer_mut(&mut self, principal_layer: usize) -> &mut Ratsnest {
        &mut self.0[principal_layer]
    }
}
