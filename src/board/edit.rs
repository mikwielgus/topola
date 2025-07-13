// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeMap;

use crate::{drawing::band::BandUid, layout::LayoutEdit};

use super::BandName;

#[derive(Debug, Clone)]
pub struct BoardDataEdit {
    pub(super) bands: BTreeMap<BandName, (Option<BandUid>, Option<BandUid>)>,
}

impl BoardDataEdit {
    pub fn new() -> Self {
        Self {
            bands: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BoardEdit {
    pub data_edit: BoardDataEdit,
    pub layout_edit: LayoutEdit,
}

impl BoardEdit {
    pub fn new() -> Self {
        Self {
            data_edit: BoardDataEdit::new(),
            layout_edit: LayoutEdit::new(),
        }
    }

    pub fn new_from_edits(data_edit: BoardDataEdit, layout_edit: LayoutEdit) -> Self {
        Self {
            data_edit,
            layout_edit,
        }
    }

    pub fn reverse_inplace(&mut self) {
        self.data_edit
            .bands
            .values_mut()
            .for_each(Self::swap_tuple_inplace);
        self.layout_edit.reverse_inplace();
    }

    fn swap_tuple_inplace<D>(x: &mut (D, D)) {
        core::mem::swap(&mut x.0, &mut x.1);
    }

    pub fn reverse(&self) -> Self {
        let mut rev = self.clone();
        rev.reverse_inplace();
        rev
    }
}
