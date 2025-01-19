// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::collections::btree_map::Entry as BTreeMapEntry;

use geo::Point;
use petgraph::stable_graph::StableDiGraph;
use rstar::RTree;

use crate::graph::{GenericIndex, GetPetgraphIndex};

use super::{
    compound::ManageCompounds,
    edit::{ApplyGeometryEdit, GeometryEdit},
    with_rtree::{BboxedIndex, GeometryWithRtree},
    AccessBendWeight, AccessDotWeight, AccessSegWeight, GenericNode, Geometry, GeometryLabel,
    GetLayer, GetWidth, Retag,
};

#[derive(Debug)]
pub struct RecordingGeometryWithRtree<PW, DW, SW, BW, CW, PI, DI, SI, BI> {
    geometry_with_rtree: GeometryWithRtree<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
}

impl<
        PW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<Index = PI> + Copy,
        DW: AccessDotWeight + Into<PW> + GetLayer,
        SW: AccessSegWeight + Into<PW> + GetLayer,
        BW: AccessBendWeight + Into<PW> + GetLayer,
        CW: Copy,
        PI: GetPetgraphIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Eq + Ord + Copy,
        DI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
        SI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
        BI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
    > RecordingGeometryWithRtree<PW, DW, SW, BW, CW, PI, DI, SI, BI>
{
    pub fn new(layer_count: usize) -> Self {
        Self {
            geometry_with_rtree: GeometryWithRtree::<PW, DW, SW, BW, CW, PI, DI, SI, BI>::new(
                layer_count,
            ),
        }
    }

    pub fn add_dot<W: AccessDotWeight + Into<PW> + GetLayer>(
        &mut self,
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
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
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
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
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
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
                    (from, to, core),
                    weight.into().try_into().unwrap_or_else(|_| unreachable!()),
                )),
            ),
        );
        bend
    }

    pub fn add_compound(
        &mut self,
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
        weight: CW,
    ) -> GenericIndex<CW> {
        let compound = self.geometry_with_rtree.add_compound(weight);
        recorder
            .compounds
            .insert(compound, (None, Some((vec![], weight))));
        compound
    }

    pub fn add_to_compound<W>(
        &mut self,
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
        primitive: GenericIndex<W>,
        compound: GenericIndex<CW>,
    ) {
        let geometry = self.geometry_with_rtree.geometry();
        let old_members = geometry.compound_members(compound).collect();
        let old_weight = geometry.compound_weight(compound);

        self.geometry_with_rtree
            .add_to_compound(primitive, compound);

        let geometry = self.geometry_with_rtree.geometry();
        let new_members = geometry.compound_members(compound).collect();
        let new_weight = geometry.compound_weight(compound);

        recorder
            .compounds
            .entry(compound)
            .or_insert((Some((old_members, old_weight)), None))
            .1 = Some((new_members, new_weight));
    }

    pub fn remove_dot(
        &mut self,
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
        dot: DI,
    ) -> Result<(), ()> {
        let weight = self.geometry_with_rtree.geometry().dot_weight(dot);
        self.geometry_with_rtree.remove_dot(dot)?;
        edit_remove_from_map(&mut recorder.dots, dot, weight);
        Ok(())
    }

    pub fn remove_seg(
        &mut self,
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
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
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
        bend: BI,
    ) {
        let geometry = self.geometry_with_rtree.geometry();
        let weight = geometry.bend_weight(bend);
        let joints = geometry.bend_joints(bend);
        let core = geometry.core(bend);
        self.geometry_with_rtree.remove_bend(bend);
        edit_remove_from_map(
            &mut recorder.bends,
            bend,
            ((joints.0, joints.1, core), weight),
        );
    }

    pub fn remove_compound(
        &mut self,
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
        compound: GenericIndex<CW>,
    ) {
        let geometry = self.geometry_with_rtree.geometry();
        let weight = geometry.compound_weight(compound);
        let members = geometry.compound_members(compound).collect();
        self.geometry_with_rtree.remove_compound(compound);
        edit_remove_from_map(&mut recorder.compounds, compound, (members, weight));
    }

    pub fn move_dot(
        &mut self,
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
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
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
        bend: BI,
        f: F,
    ) where
        F: FnOnce(&mut GeometryWithRtree<PW, DW, SW, BW, CW, PI, DI, SI, BI>, BI),
    {
        let geometry = self.geometry_with_rtree.geometry();
        let old_joints = geometry.bend_joints(bend);
        let old_core = geometry.core(bend);
        let old_weight = geometry.bend_weight(bend);

        f(&mut self.geometry_with_rtree, bend);

        let geometry = self.geometry_with_rtree.geometry();
        let new_joints = geometry.bend_joints(bend);
        let new_core = geometry.core(bend);
        let new_weight = geometry.bend_weight(bend);

        recorder
            .bends
            .entry(bend)
            .or_insert((
                Some(((old_joints.0, old_joints.1, old_core), old_weight)),
                None,
            ))
            .1 = Some(((new_joints.0, new_joints.1, new_core), new_weight));
    }

    pub fn shift_bend(
        &mut self,
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
        bend: BI,
        offset: f64,
    ) {
        self.modify_bend(recorder, bend, |geometry_with_rtree, bend| {
            geometry_with_rtree.shift_bend(bend, offset)
        });
    }

    pub fn flip_bend(
        &mut self,
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
        bend: BI,
    ) {
        self.modify_bend(recorder, bend, |geometry_with_rtree, bend| {
            geometry_with_rtree.flip_bend(bend)
        });
    }

    pub fn reattach_bend(
        &mut self,
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
        bend: BI,
        maybe_new_inner: Option<BI>,
    ) {
        self.modify_bend(recorder, bend, |geometry_with_rtree, bend| {
            geometry_with_rtree.reattach_bend(bend, maybe_new_inner)
        });
    }

    pub fn compound_weight(&self, compound: GenericIndex<CW>) -> CW {
        self.geometry_with_rtree.compound_weight(compound)
    }

    pub fn compounds<'a, W: 'a>(
        &'a self,
        node: GenericIndex<W>,
    ) -> impl Iterator<Item = GenericIndex<CW>> + 'a {
        self.geometry_with_rtree.compounds(node)
    }

    pub fn geometry(&self) -> &Geometry<PW, DW, SW, BW, CW, PI, DI, SI, BI> {
        self.geometry_with_rtree.geometry()
    }

    pub fn rtree(&self) -> &RTree<BboxedIndex<GenericNode<PI, GenericIndex<CW>>>> {
        self.geometry_with_rtree.rtree()
    }

    pub fn layer_count(&self) -> usize {
        *self.geometry_with_rtree.layer_count()
    }

    pub fn graph(&self) -> &StableDiGraph<GenericNode<PW, CW>, GeometryLabel, usize> {
        self.geometry_with_rtree.graph()
    }
}

fn edit_remove_from_map<I: Ord, T>(
    map: &mut std::collections::BTreeMap<I, (Option<T>, Option<T>)>,
    index: I,
    data: T,
) where
    I: core::cmp::Eq + Ord,
{
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
        CW: Copy,
        PI: GetPetgraphIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Eq + Ord + Copy,
        DI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
        SI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
        BI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
    > ApplyGeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>
    for RecordingGeometryWithRtree<PW, DW, SW, BW, CW, PI, DI, SI, BI>
{
    fn apply(&mut self, edit: &GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>) {
        for (compound, (maybe_old_data, ..)) in &edit.compounds {
            if maybe_old_data.is_some() {
                self.geometry_with_rtree.remove_compound(*compound);
            }
        }

        for (bend, (maybe_old_data, ..)) in &edit.bends {
            if maybe_old_data.is_some() {
                self.geometry_with_rtree.remove_bend(*bend);
            }
        }

        for (seg, (maybe_old_data, ..)) in &edit.segs {
            if maybe_old_data.is_some() {
                self.geometry_with_rtree.remove_seg(*seg);
            }
        }

        for (dot, (maybe_old_data, ..)) in &edit.dots {
            if maybe_old_data.is_some() {
                self.geometry_with_rtree.remove_dot(*dot);
            }
        }

        for (dot, (.., maybe_new_data)) in &edit.dots {
            if let Some(weight) = maybe_new_data {
                self.geometry_with_rtree.add_dot_at_index(*dot, *weight);
            }
        }

        for (seg, (.., maybe_new_data)) in &edit.segs {
            if let Some(((from, to), weight)) = maybe_new_data {
                self.geometry_with_rtree
                    .add_seg_at_index(*seg, *from, *to, *weight);
            }
        }

        for (bend, (.., maybe_new_data)) in &edit.bends {
            if let Some(((from, to, core), weight)) = maybe_new_data {
                self.geometry_with_rtree
                    .add_bend_at_index(*bend, *from, *to, *core, *weight);
            }
        }

        for (compound, (.., maybe_new_data)) in &edit.compounds {
            if let Some((members, weight)) = maybe_new_data {
                self.geometry_with_rtree
                    .add_compound_at_index(*compound, *weight);

                for member in members {
                    self.geometry_with_rtree.add_to_compound(
                        GenericIndex::<PW>::new(member.petgraph_index()),
                        *compound,
                    );
                }
            }
        }
    }
}
