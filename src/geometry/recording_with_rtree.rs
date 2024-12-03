use std::hash::Hash;

use geo::Point;
use petgraph::stable_graph::StableDiGraph;
use rstar::RTree;

use crate::{
    drawing::graph::{GetLayer, Retag},
    graph::{GenericIndex, GetPetgraphIndex},
};

use super::{
    compound::ManageCompounds,
    edit::{ApplyGeometryEdit, GeometryEdit},
    with_rtree::{BboxedIndex, GeometryWithRtree},
    AccessBendWeight, AccessDotWeight, AccessSegWeight, GenericNode, Geometry, GeometryLabel,
    GetWidth,
};

#[derive(Debug)]
pub struct RecordingGeometryWithRtree<
    PW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<PI> + Copy,
    DW: AccessDotWeight<PW> + GetLayer,
    SW: AccessSegWeight<PW> + GetLayer,
    BW: AccessBendWeight<PW> + GetLayer,
    CW: Copy,
    PI: GetPetgraphIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Eq + Hash + Copy,
    DI: GetPetgraphIndex + Into<PI> + Eq + Hash + Copy,
    SI: GetPetgraphIndex + Into<PI> + Eq + Hash + Copy,
    BI: GetPetgraphIndex + Into<PI> + Eq + Hash + Copy,
> {
    geometry_with_rtree: GeometryWithRtree<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
}

impl<
        PW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<PI> + Copy,
        DW: AccessDotWeight<PW> + GetLayer,
        SW: AccessSegWeight<PW> + GetLayer,
        BW: AccessBendWeight<PW> + GetLayer,
        CW: Copy,
        PI: GetPetgraphIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Eq + Hash + Copy,
        DI: GetPetgraphIndex + Into<PI> + Eq + Hash + Copy,
        SI: GetPetgraphIndex + Into<PI> + Eq + Hash + Copy,
        BI: GetPetgraphIndex + Into<PI> + Eq + Hash + Copy,
    > RecordingGeometryWithRtree<PW, DW, SW, BW, CW, PI, DI, SI, BI>
{
    pub fn new(layer_count: usize) -> Self {
        Self {
            geometry_with_rtree: GeometryWithRtree::<PW, DW, SW, BW, CW, PI, DI, SI, BI>::new(
                layer_count,
            ),
        }
    }

    pub fn add_dot<W: AccessDotWeight<PW> + GetLayer>(
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

    pub fn add_seg<W: AccessSegWeight<PW> + GetLayer>(
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

    pub fn add_bend<W: AccessBendWeight<PW> + GetLayer>(
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
        let old_members = self
            .geometry_with_rtree
            .geometry()
            .compound_members(compound)
            .collect();
        let old_weight = self
            .geometry_with_rtree
            .geometry()
            .compound_weight(compound);

        let new_members = self
            .geometry_with_rtree
            .geometry()
            .compound_members(compound)
            .collect();
        let new_weight = self
            .geometry_with_rtree
            .geometry()
            .compound_weight(compound);

        if let Some(value) = recorder.compounds.get_mut(&compound) {
            value.1 = Some((new_members, new_weight));
        } else {
            recorder.compounds.insert(
                compound,
                (
                    Some((old_members, old_weight)),
                    Some((new_members, new_weight)),
                ),
            );
        }
    }

    pub fn remove_dot(
        &mut self,
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
        dot: DI,
    ) -> Result<(), ()> {
        let weight = self.geometry_with_rtree.geometry().dot_weight(dot);
        self.geometry_with_rtree.remove_dot(dot)?;

        if let Some((None, Some(..))) = recorder.dots.get(&dot) {
            recorder.dots.remove(&dot);
        } else {
            recorder.dots.insert(dot, (Some(weight), None));
        };

        Ok(())
    }

    pub fn remove_seg(
        &mut self,
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
        seg: SI,
    ) {
        let weight = self.geometry_with_rtree.geometry().seg_weight(seg);
        let joints = self.geometry_with_rtree.geometry().seg_joints(seg);
        self.geometry_with_rtree.remove_seg(seg);

        if let Some((None, Some(..))) = recorder.segs.get(&seg) {
            recorder.segs.remove(&seg);
        } else {
            recorder.segs.insert(seg, (Some((joints, weight)), None));
        }
    }

    pub fn remove_bend(
        &mut self,
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
        bend: BI,
    ) {
        let weight = self.geometry_with_rtree.geometry().bend_weight(bend);
        let joints = self.geometry_with_rtree.geometry().bend_joints(bend);
        let core = self.geometry_with_rtree.geometry().core(bend);
        self.geometry_with_rtree.remove_bend(bend);

        if let Some((None, Some(..))) = recorder.bends.get(&bend) {
            recorder.bends.remove(&bend);
        } else {
            recorder
                .bends
                .insert(bend, (Some(((joints.0, joints.1, core), weight)), None));
        }
    }

    pub fn remove_compound(
        &mut self,
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
        compound: GenericIndex<CW>,
    ) {
        let weight = self
            .geometry_with_rtree
            .geometry()
            .compound_weight(compound);
        let members = self
            .geometry_with_rtree
            .geometry()
            .compound_members(compound)
            .collect();
        self.geometry_with_rtree.remove_compound(compound);

        if let Some((None, Some(..))) = recorder.compounds.get(&compound) {
            recorder.compounds.remove(&compound);
        } else {
            recorder
                .compounds
                .insert(compound, (Some((members, weight)), None));
        }
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

        if let Some(value) = recorder.dots.get_mut(&dot) {
            value.1 = Some(new_weight);
        } else {
            recorder
                .dots
                .insert(dot, (Some(old_weight), Some(new_weight)));
        }
    }

    pub fn shift_bend(
        &mut self,
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
        bend: BI,
        offset: f64,
    ) {
        let old_joints = self.geometry_with_rtree.geometry().bend_joints(bend);
        let old_core = self.geometry_with_rtree.geometry().core(bend);
        let old_weight = self.geometry_with_rtree.geometry().bend_weight(bend);
        self.geometry_with_rtree.shift_bend(bend, offset);
        let new_joints = self.geometry_with_rtree.geometry().bend_joints(bend);
        let new_core = self.geometry_with_rtree.geometry().core(bend);
        let new_weight = self.geometry_with_rtree.geometry().bend_weight(bend);

        if let Some(value) = recorder.bends.get_mut(&bend) {
            value.1 = Some(((new_joints.0, new_joints.1, new_core), new_weight));
        } else {
            recorder.bends.insert(
                bend,
                (
                    Some(((old_joints.0, old_joints.1, old_core), old_weight)),
                    Some(((new_joints.0, new_joints.1, new_core), new_weight)),
                ),
            );
        }
    }

    pub fn flip_bend(
        &mut self,
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
        bend: BI,
    ) {
        let old_joints = self.geometry_with_rtree.geometry().bend_joints(bend);
        let old_core = self.geometry_with_rtree.geometry().core(bend);
        let old_weight = self.geometry_with_rtree.geometry().bend_weight(bend);
        self.geometry_with_rtree.flip_bend(bend);
        let new_joints = self.geometry_with_rtree.geometry().bend_joints(bend);
        let new_core = self.geometry_with_rtree.geometry().core(bend);
        let new_weight = self.geometry_with_rtree.geometry().bend_weight(bend);

        if let Some(value) = recorder.bends.get_mut(&bend) {
            value.1 = Some(((new_joints.0, new_joints.1, new_core), new_weight));
        } else {
            recorder.bends.insert(
                bend,
                (
                    Some(((old_joints.0, old_joints.1, old_core), old_weight)),
                    Some(((new_joints.0, new_joints.1, new_core), new_weight)),
                ),
            );
        }
    }

    pub fn reattach_bend(
        &mut self,
        recorder: &mut GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>,
        bend: BI,
        maybe_new_inner: Option<BI>,
    ) {
        let old_joints = self.geometry_with_rtree.geometry().bend_joints(bend);
        let old_core = self.geometry_with_rtree.geometry().core(bend);
        let old_weight = self.geometry_with_rtree.geometry().bend_weight(bend);
        self.geometry_with_rtree
            .reattach_bend(bend, maybe_new_inner);
        let new_joints = self.geometry_with_rtree.geometry().bend_joints(bend);
        let new_core = self.geometry_with_rtree.geometry().core(bend);
        let new_weight = self.geometry_with_rtree.geometry().bend_weight(bend);

        if let Some(value) = recorder.bends.get_mut(&bend) {
            value.1 = Some(((new_joints.0, new_joints.1, new_core), new_weight));
        } else {
            recorder.bends.insert(
                bend,
                (
                    Some(((old_joints.0, old_joints.1, old_core), old_weight)),
                    Some(((new_joints.0, new_joints.1, new_core), new_weight)),
                ),
            );
        }
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

impl<
        PW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<PI> + Copy,
        DW: AccessDotWeight<PW> + GetLayer,
        SW: AccessSegWeight<PW> + GetLayer,
        BW: AccessBendWeight<PW> + GetLayer,
        CW: Copy,
        PI: GetPetgraphIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Eq + Hash + Copy,
        DI: GetPetgraphIndex + Into<PI> + Eq + Hash + Copy,
        SI: GetPetgraphIndex + Into<PI> + Eq + Hash + Copy,
        BI: GetPetgraphIndex + Into<PI> + Eq + Hash + Copy,
    > ApplyGeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>
    for RecordingGeometryWithRtree<PW, DW, SW, BW, CW, PI, DI, SI, BI>
{
    fn apply(&mut self, edit: GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>) {
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
