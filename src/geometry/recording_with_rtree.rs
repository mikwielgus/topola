// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::collections::btree_map::Entry as BTreeMapEntry;

use geo::Point;
use rstar::RTree;

use crate::graph::{GenericIndex, GetIndex};

use super::{
    compound::ManageCompounds,
    edit::{ApplyGeometryEdit, GeometryEdit},
    with_rtree::{BboxedIndex, GeometryWithRtree},
    AccessBendWeight, AccessDotWeight, AccessSegWeight, GenericNode, Geometry, GetLayer, GetWidth,
    Retag,
};

#[derive(Debug)]
pub struct RecordingGeometryWithRtree<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI> {
    geometry_with_rtree: GeometryWithRtree<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>,
}

impl<PW: Clone, DW, SW, BW, CW: Clone, Cel: Clone, PI: Clone, DI, SI, BI> Clone
    for RecordingGeometryWithRtree<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>
{
    fn clone(&self) -> Self {
        Self {
            geometry_with_rtree: self.geometry_with_rtree.clone(),
        }
    }
}

impl<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>
    RecordingGeometryWithRtree<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>
{
    pub fn geometry(&self) -> &Geometry<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI> {
        self.geometry_with_rtree.geometry()
    }

    pub fn rtree(&self) -> &RTree<BboxedIndex<GenericNode<PI, GenericIndex<CW>>>> {
        self.geometry_with_rtree.rtree()
    }

    pub fn layer_count(&self) -> usize {
        *self.geometry_with_rtree.layer_count()
    }

    pub fn node_count(&self) -> usize {
        self.geometry_with_rtree.node_count()
    }
}

impl<
        PW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<Index = PI> + Copy,
        DW: AccessDotWeight + Into<PW> + GetLayer,
        SW: AccessSegWeight + Into<PW> + GetLayer,
        BW: AccessBendWeight + Into<PW> + GetLayer,
        CW: Clone,
        Cel: Copy,
        PI: GetIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Eq + Ord + Copy,
        DI: GetIndex + Into<PI> + Eq + Ord + Copy,
        SI: GetIndex + Into<PI> + Eq + Ord + Copy,
        BI: GetIndex + Into<PI> + Eq + Ord + Copy,
    > RecordingGeometryWithRtree<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>
{
    pub fn new(layer_count: usize) -> Self {
        Self {
            geometry_with_rtree: GeometryWithRtree::new(layer_count),
        }
    }

    pub fn add_dot<W: AccessDotWeight + Into<PW> + GetLayer>(
        &mut self,
        recorder: &mut GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>,
        weight: W,
    ) -> GenericIndex<W>
    where
        GenericIndex<W>: Into<PI>,
    {
        let dot = self.geometry_with_rtree.add_dot(weight);
        recorder.dots.insert(
            Into::<PI>::into(dot)
                .try_into()
                .unwrap_or_else(|_| unreachable!()),
            (
                None,
                Some(weight.into().try_into().unwrap_or_else(|_| unreachable!())),
            ),
        );
        dot
    }

    pub fn add_seg<W: AccessSegWeight + Into<PW> + GetLayer>(
        &mut self,
        recorder: &mut GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>,
        from: DI,
        to: DI,
        weight: W,
    ) -> GenericIndex<W>
    where
        GenericIndex<W>: Into<PI>,
    {
        let seg = self.geometry_with_rtree.add_seg(from, to, weight);
        recorder.segs.insert(
            Into::<PI>::into(seg)
                .try_into()
                .unwrap_or_else(|_| unreachable!()),
            (
                None,
                Some((
                    (from, to),
                    weight.into().try_into().unwrap_or_else(|_| unreachable!()),
                )),
            ),
        );
        seg
    }

    pub fn add_bend<W: AccessBendWeight + Into<PW> + GetLayer>(
        &mut self,
        recorder: &mut GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>,
        from: DI,
        to: DI,
        core: DI,
        weight: W,
    ) -> GenericIndex<W>
    where
        GenericIndex<W>: Into<PI>,
    {
        let bend = self.geometry_with_rtree.add_bend(from, to, core, weight);
        recorder.bends.insert(
            Into::<PI>::into(bend)
                .try_into()
                .unwrap_or_else(|_| unreachable!()),
            (
                None,
                Some((
                    (from, to, core, None),
                    weight.into().try_into().unwrap_or_else(|_| unreachable!()),
                )),
            ),
        );
        bend
    }

    pub fn add_compound(
        &mut self,
        recorder: &mut GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>,
        weight: CW,
    ) -> GenericIndex<CW> {
        let compound = self.geometry_with_rtree.add_compound(weight.clone());
        recorder
            .compounds
            .insert(compound, (None, Some((vec![], weight))));
        compound
    }

    pub fn add_to_compound<W>(
        &mut self,
        recorder: &mut GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>,
        primitive: GenericIndex<W>,
        entry_label: Cel,
        compound: GenericIndex<CW>,
    ) {
        let geometry = self.geometry_with_rtree.geometry();
        let old_members = geometry.compound_members(compound).collect();
        let old_weight = geometry.compound_weight(compound).clone();

        self.geometry_with_rtree
            .add_to_compound(primitive, entry_label, compound);

        let geometry = self.geometry_with_rtree.geometry();
        let new_members = geometry.compound_members(compound).collect();
        let new_weight = geometry.compound_weight(compound).clone();

        recorder
            .compounds
            .entry(compound)
            .or_insert((Some((old_members, old_weight)), None))
            .1 = Some((new_members, new_weight));
    }

    pub fn remove_dot(
        &mut self,
        recorder: &mut GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>,
        dot: DI,
    ) -> Result<(), ()> {
        let weight = self.geometry_with_rtree.geometry().dot_weight(dot);
        self.geometry_with_rtree.remove_dot(dot);
        edit_remove_from_map(&mut recorder.dots, dot, weight);
        Ok(())
    }

    pub fn remove_seg(
        &mut self,
        recorder: &mut GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>,
        seg: SI,
    ) {
        let geometry = self.geometry_with_rtree.geometry();
        let weight = geometry.seg_weight(seg);
        let joints = geometry.seg_joints(seg);
        self.geometry_with_rtree.remove_seg(seg);
        edit_remove_from_map(&mut recorder.segs, seg, (joints, weight));
    }

    pub fn remove_bend(
        &mut self,
        recorder: &mut GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>,
        bend: BI,
    ) {
        let geometry = self.geometry_with_rtree.geometry();
        let weight = geometry.bend_weight(bend);
        let joints = geometry.bend_joints(bend);
        let core = geometry.core(bend);
        let maybe_inner = geometry.inner(bend);

        self.geometry_with_rtree.remove_bend(bend);
        edit_remove_from_map(
            &mut recorder.bends,
            bend,
            ((joints.0, joints.1, core, maybe_inner), weight),
        );
    }

    pub fn remove_compound(
        &mut self,
        recorder: &mut GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>,
        compound: GenericIndex<CW>,
    ) {
        let geometry = self.geometry_with_rtree.geometry();
        let weight = geometry.compound_weight(compound).clone();
        let members = geometry.compound_members(compound).collect();
        self.geometry_with_rtree.remove_compound(compound);
        edit_remove_from_map(&mut recorder.compounds, compound, (members, weight));
    }

    pub fn move_dot(
        &mut self,
        recorder: &mut GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>,
        dot: DI,
        to: Point,
    ) {
        let old_weight = self.geometry_with_rtree.geometry().dot_weight(dot);
        self.geometry_with_rtree.move_dot(dot, to);
        let new_weight = self.geometry_with_rtree.geometry().dot_weight(dot);

        recorder
            .dots
            .entry(dot)
            .or_insert((Some(old_weight), None))
            .1 = Some(new_weight);
    }

    fn modify_bend<F>(
        &mut self,
        recorder: &mut GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>,
        bend: BI,
        f: F,
    ) where
        F: FnOnce(&mut GeometryWithRtree<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>, BI),
    {
        let geometry = self.geometry_with_rtree.geometry();
        let old_joints = geometry.bend_joints(bend);
        let old_core = geometry.core(bend);
        let old_maybe_inner = geometry.inner(bend);
        let old_weight = geometry.bend_weight(bend);

        f(&mut self.geometry_with_rtree, bend);

        let geometry = self.geometry_with_rtree.geometry();
        let new_joints = geometry.bend_joints(bend);
        let new_core = geometry.core(bend);
        let new_maybe_inner = geometry.inner(bend);
        let new_weight = geometry.bend_weight(bend);

        recorder
            .bends
            .entry(bend)
            .or_insert((
                Some((
                    (old_joints.0, old_joints.1, old_core, old_maybe_inner),
                    old_weight,
                )),
                None,
            ))
            .1 = Some((
            (new_joints.0, new_joints.1, new_core, new_maybe_inner),
            new_weight,
        ));
    }

    pub fn shift_bend(
        &mut self,
        recorder: &mut GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>,
        bend: BI,
        offset: f64,
    ) {
        self.modify_bend(recorder, bend, |geometry_with_rtree, bend| {
            geometry_with_rtree.shift_bend(bend, offset)
        });
    }

    pub fn flip_bend(
        &mut self,
        recorder: &mut GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>,
        bend: BI,
    ) {
        self.modify_bend(recorder, bend, |geometry_with_rtree, bend| {
            geometry_with_rtree.flip_bend(bend)
        });
    }

    pub fn reattach_bend(
        &mut self,
        recorder: &mut GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>,
        bend: BI,
        maybe_new_inner: Option<BI>,
    ) {
        self.modify_bend(recorder, bend, |geometry_with_rtree, bend| {
            geometry_with_rtree.reattach_bend(bend, maybe_new_inner)
        });
    }

    pub fn compound_weight(&self, compound: GenericIndex<CW>) -> &CW {
        self.geometry_with_rtree.compound_weight(compound)
    }

    pub fn compounds<'a, W: 'a>(
        &'a self,
        node: GenericIndex<W>,
    ) -> impl Iterator<Item = (Cel, GenericIndex<CW>)> + 'a {
        self.geometry_with_rtree.compounds(node)
    }
}

fn edit_remove_from_map<I: Eq + Ord, T>(
    map: &mut std::collections::BTreeMap<I, (Option<T>, Option<T>)>,
    index: I,
    data: T,
) {
    let to_be_inserted = (Some(data), None);
    match map.entry(index) {
        BTreeMapEntry::Occupied(mut occ) => {
            if let (None, Some(_)) = occ.get() {
                occ.remove();
            } else {
                *occ.get_mut() = to_be_inserted;
            }
        }
        BTreeMapEntry::Vacant(vac) => {
            vac.insert(to_be_inserted);
        }
    }
}

impl<
        PW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<Index = PI> + Copy,
        DW: AccessDotWeight + Into<PW> + GetLayer,
        SW: AccessSegWeight + Into<PW> + GetLayer,
        BW: AccessBendWeight + Into<PW> + GetLayer,
        CW: Clone,
        Cel: Copy,
        PI: GetIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Eq + Ord + Copy,
        DI: GetIndex + Into<PI> + Eq + Ord + Copy,
        SI: GetIndex + Into<PI> + Eq + Ord + Copy,
        BI: GetIndex + Into<PI> + Eq + Ord + Copy,
    > ApplyGeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>
    for RecordingGeometryWithRtree<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>
{
    fn apply(&mut self, edit: &GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>) {
        self.geometry_with_rtree.apply(edit);
    }
}
