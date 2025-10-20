// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use spade::InsertionError;
use specctra_core::mesadata::AccessMesadata;

use crate::{autorouter::ratsnest::Ratsnest, board::Board};

pub struct Ratsnests(Box<[Ratsnest]>);

impl Ratsnests {
    pub fn new(board: &Board<impl AccessMesadata>) -> Result<Self, InsertionError> {
        Ok(Self(
            (0..board.layout().drawing().layer_count())
                .map(|principal_layer| Ratsnest::new(board, principal_layer))
                .collect::<Result<Vec<_>, _>>()
                .map(Vec::into_boxed_slice)?,
        ))
    }

    pub fn on_principal_layer(&self, principal_layer: usize) -> &Ratsnest {
        &self.0[principal_layer]
    }

    pub fn on_principal_layer_mut(&mut self, principal_layer: usize) -> &mut Ratsnest {
        &mut self.0[principal_layer]
    }
}
