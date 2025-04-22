// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use crate::graph::{GenericIndex, GetPetgraphIndex};

pub trait ManageCompounds<CW: Clone> {
    type GeneralIndex: Copy;
    type EntryKind: Copy;

    fn add_compound(&mut self, weight: CW) -> GenericIndex<CW>;
    fn remove_compound(&mut self, compound: GenericIndex<CW>);
    fn add_to_compound<I>(&mut self, node: I, kind: Self::EntryKind, compound: GenericIndex<CW>)
    where
        I: Copy + GetPetgraphIndex;

    fn compound_weight(&self, node: GenericIndex<CW>) -> &CW;

    fn compound_members(
        &self,
        compound: GenericIndex<CW>,
    ) -> impl Iterator<Item = (Self::EntryKind, Self::GeneralIndex)> + '_;

    fn compounds<I>(&self, node: I) -> impl Iterator<Item = (Self::EntryKind, GenericIndex<CW>)>
    where
        I: Copy + GetPetgraphIndex;
}
