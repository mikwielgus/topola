// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::{
    board::BandName,
    drawing::band::BandUid,
    geometry::edit::Edit,
    layout::{LayoutEdit, NodeIndex},
    router::ng::EtchedPath,
};

#[derive(Debug, Clone, Default)]
pub struct BoardDataEdit {
    pub(super) pinname_nodes: BTreeMap<NodeIndex, (Option<String>, Option<String>)>,
    pub(super) bands_by_id: BTreeMap<EtchedPath, (Option<BandUid>, Option<BandUid>)>,
    pub(super) bands_by_name: BTreeMap<BandName, (Option<BandUid>, Option<BandUid>)>,
}

impl BoardDataEdit {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Edit for BoardDataEdit {
    fn reverse_inplace(&mut self) {
        self.pinname_nodes.reverse_inplace();
        self.bands_by_id.reverse_inplace();
        self.bands_by_name.reverse_inplace();
    }

    fn merge(&mut self, edit: Self) {
        self.pinname_nodes.merge(edit.pinname_nodes);
        self.bands_by_id.merge(edit.bands_by_id);
        self.bands_by_name.merge(edit.bands_by_name);
    }
}

#[derive(Debug, Clone, Default)]
pub struct BoardEdit {
    pub board_data_edit: BoardDataEdit,
    pub layout_edit: LayoutEdit,
}

impl BoardEdit {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_from_edits(data_edit: BoardDataEdit, layout_edit: LayoutEdit) -> Self {
        Self {
            board_data_edit: data_edit,
            layout_edit,
        }
    }
}

impl Edit for BoardEdit {
    fn reverse_inplace(&mut self) {
        self.board_data_edit.reverse_inplace();
        self.layout_edit.reverse_inplace();
    }

    fn merge(&mut self, edit: Self) {
        self.board_data_edit.merge(edit.board_data_edit);
        self.layout_edit.merge(edit.layout_edit);
    }
}
