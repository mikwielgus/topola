// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use contracts_try::debug_invariant;
use derive_getters::Getters;
use geo::Point;
use petgraph::{stable_graph::StableDiGraph, visit::Walker};
use rstar::{primitives::GeomWithData, Envelope, RTree, RTreeObject, AABB};

use crate::{
    geometry::{
        compound::ManageCompounds,
        primitive::{AccessPrimitiveShape, PrimitiveShape},
        AccessBendWeight, AccessDotWeight, AccessSegWeight, GenericNode, Geometry, GeometryLabel,
        GetLayer, GetWidth, Retag,
    },
    graph::{GenericIndex, GetPetgraphIndex},
};

use super::edit::{ApplyGeometryEdit, GeometryEdit};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bbox {
    pub aabb: AABB<[f64; 3]>,
}

impl Bbox {
    pub fn new(aabb: AABB<[f64; 3]>) -> Bbox {
        Self { aabb }
    }
}

impl RTreeObject for Bbox {
    type Envelope = AABB<[f64; 3]>;
    fn envelope(&self) -> Self::Envelope {
        self.aabb
    }
}

pub type BboxedIndex<I> = GeomWithData<Bbox, I>;

#[derive(Debug, Getters)]
pub struct GeometryWithRtree<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI> {
    geometry: Geometry<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>,
    rtree: RTree<BboxedIndex<GenericNode<PI, GenericIndex<CW>>>>,
    layer_count: usize,
}

impl<PW: Clone, DW, SW, BW, CW: Clone, Cel: Clone, PI: Clone, DI, SI, BI> Clone
    for GeometryWithRtree<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>
{
    fn clone(&self) -> Self {
        Self {
            geometry: self.geometry.clone(),
            rtree: self.rtree.clone(),
            layer_count: self.layer_count,
        }
    }
}

impl<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>
    GeometryWithRtree<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>
{
    pub fn graph(&self) -> &StableDiGraph<GenericNode<PW, CW>, GeometryLabel<Cel>, usize> {
        self.geometry.graph()
    }
}

#[debug_invariant(self.test_envelopes())]
#[debug_invariant(self.geometry.graph().node_count() == self.rtree.size())]
impl<
        PW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<Index = PI> + Copy,
        DW: AccessDotWeight + Into<PW> + GetLayer,
        SW: AccessSegWeight + Into<PW> + GetLayer,
        BW: AccessBendWeight + Into<PW> + GetLayer,
        CW: Clone,
        Cel: Copy,
        PI: GetPetgraphIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + PartialEq + Copy,
        DI: GetPetgraphIndex + Into<PI> + Copy,
        SI: GetPetgraphIndex + Into<PI> + Copy,
        BI: GetPetgraphIndex + Into<PI> + Copy,
    > GeometryWithRtree<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>
{
    pub fn new(layer_count: usize) -> Self {
        Self {
            geometry: Geometry::new(),
            rtree: RTree::new(),
            layer_count,
        }
    }

    pub fn add_dot<W: AccessDotWeight + Into<PW> + GetLayer>(
        &mut self,
        weight: W,
    ) -> GenericIndex<W>
    where
        GenericIndex<W>: Into<PI>,
    {
        let dot = self.geometry.add_dot(weight);
        self.init_dot_bbox(dot.into().try_into().unwrap_or_else(|_| unreachable!()));
        dot
    }

    pub(super) fn add_dot_at_index<W: AccessDotWeight + Into<PW> + GetLayer>(
        &mut self,
        dot: DI,
        weight: W,
    ) {
        self.geometry
            .add_dot_at_index(GenericIndex::<W>::new(dot.petgraph_index()), weight);
        self.init_dot_bbox(dot);
    }

    pub fn add_seg<W: AccessSegWeight + Into<PW> + GetLayer>(
        &mut self,
        from: DI,
        to: DI,
        weight: W,
    ) -> GenericIndex<W>
    where
        GenericIndex<W>: Into<PI>,
    {
        let seg = self.geometry.add_seg(from, to, weight);
        self.init_seg_bbox(seg.into().try_into().unwrap_or_else(|_| unreachable!()));
        seg
    }

    pub(super) fn add_seg_at_index<W: AccessSegWeight + Into<PW> + GetLayer>(
        &mut self,
        seg: SI,
        from: DI,
        to: DI,
        weight: W,
    ) {
        self.geometry.add_seg_at_index(
            GenericIndex::<W>::new(seg.petgraph_index()),
            from,
            to,
            weight,
        );
        self.init_seg_bbox(seg);
    }

    pub fn add_bend<W: AccessBendWeight + Into<PW> + GetLayer>(
        &mut self,
        from: DI,
        to: DI,
        core: DI,
        weight: W,
    ) -> GenericIndex<W>
    where
        GenericIndex<W>: Into<PI>,
    {
        let bend = self.geometry.add_bend(from, to, core, weight);
        self.init_bend_bbox(bend.into().try_into().unwrap_or_else(|_| unreachable!()));
        bend
    }

    pub(super) fn add_compound_at_index(&mut self, compound: GenericIndex<CW>, weight: CW) {
        self.geometry
            .add_compound_at_index(GenericIndex::<CW>::new(compound.petgraph_index()), weight);
    }

    pub fn add_to_compound<W>(
        &mut self,
        primitive: GenericIndex<W>,
        entry_label: Cel,
        compound: GenericIndex<CW>,
    ) {
        Self::rtree_remove_must_be_successful(
            self.rtree.remove(&self.make_compound_bbox(compound)),
        );
        self.geometry
            .add_to_compound(primitive, entry_label, compound);
        self.rtree.insert(self.make_compound_bbox(compound));
    }

    pub fn remove_dot(&mut self, dot: DI) {
        debug_assert!(self.geometry.joined_segs(dot).next().is_none());
        debug_assert!(self.geometry.joined_bends(dot).next().is_none());

        Self::rtree_remove_must_be_successful(self.rtree.remove(&self.make_dot_bbox(dot)));
        self.geometry.remove_primitive(dot.into());
    }

    pub fn remove_seg(&mut self, seg: SI) {
        Self::rtree_remove_must_be_successful(self.rtree.remove(&self.make_seg_bbox(seg)));
        self.geometry.remove_primitive(seg.into());
    }

    pub fn remove_bend(&mut self, bend: BI) {
        // Note how we can't use `.modify_bend(...)` to update the envelopes
        // here because it starts iteration from the bend we currently operate
        // on, which obviously does not exist after deletion.

        let mut outwards = self.geometry.outwards(bend);
        while let Some(next) = outwards.walk_next(&self.geometry) {
            Self::rtree_remove_must_be_successful(self.rtree.remove(&self.make_bend_bbox(next)));
        }

        Self::rtree_remove_must_be_successful(self.rtree.remove(&self.make_bend_bbox(bend)));
        self.geometry.remove_primitive(bend.into());

        let mut outwards = self.geometry.outwards(bend);
        while let Some(next) = outwards.walk_next(&self.geometry) {
            self.rtree.insert(self.make_bend_bbox(next));
        }
    }

    pub fn remove_compound(&mut self, compound: GenericIndex<CW>) {
        Self::rtree_remove_must_be_successful(
            self.rtree.remove(&self.make_compound_bbox(compound)),
        );
        self.geometry.remove_compound(compound);
    }

    pub fn move_dot(&mut self, dot: DI, to: Point) {
        self.modify_dot(dot, |geometry, dot| {
            geometry.move_dot(dot, to);
        });
    }

    fn modify_dot<F>(&mut self, dot: DI, f: F)
    where
        F: FnOnce(&mut Geometry<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>, DI),
    {
        for seg in self.geometry.joined_segs(dot) {
            Self::rtree_remove_must_be_successful(self.rtree.remove(&self.make_seg_bbox(seg)));
        }

        for bend in self.geometry.joined_bends(dot) {
            Self::rtree_remove_must_be_successful(self.rtree.remove(&self.make_bend_bbox(bend)));
        }

        Self::rtree_remove_must_be_successful(self.rtree.remove(&self.make_dot_bbox(dot)));

        f(&mut self.geometry, dot);

        self.rtree.insert(self.make_dot_bbox(dot));

        for bend in self.geometry.joined_bends(dot) {
            self.rtree.insert(self.make_bend_bbox(bend));
        }

        for seg in self.geometry.joined_segs(dot) {
            self.rtree.insert(self.make_seg_bbox(seg));
        }
    }

    pub fn shift_bend(&mut self, bend: BI, offset: f64) {
        self.modify_bend(bend, |geometry, bend| {
            geometry.shift_bend(bend, offset);
        });
    }

    pub fn flip_bend(&mut self, bend: BI) {
        // Does not affect the bbox because it covers the whole guidecircle.
        self.geometry.flip_bend(bend);
    }

    pub fn reattach_bend(&mut self, bend: BI, maybe_new_inner: Option<BI>) {
        self.modify_bend(bend, |geometry, bend| {
            geometry.reattach_bend(bend, maybe_new_inner);
        });
    }

    fn modify_bend<F>(&mut self, bend: BI, f: F)
    where
        F: FnOnce(&mut Geometry<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>, BI),
    {
        let mut outwards = self.geometry.outwards(bend);
        while let Some(next) = outwards.walk_next(&self.geometry) {
            Self::rtree_remove_must_be_successful(self.rtree.remove(&self.make_bend_bbox(next)));
        }

        Self::rtree_remove_must_be_successful(self.rtree.remove(&self.make_bend_bbox(bend)));

        f(&mut self.geometry, bend);

        self.rtree.insert(self.make_bend_bbox(bend));

        let mut outwards = self.geometry.outwards(bend);
        while let Some(next) = outwards.walk_next(&self.geometry) {
            self.rtree.insert(self.make_bend_bbox(next));
        }
    }

    /// If you are wondering why this method only lengthily asserts without
    /// doing the removal itself, this is because doing the removal would
    /// require us to mutably borrow self, which would prevent us from using
    /// this function inside for loops over any of the geometry's nodes due to
    /// borrow checker's restrictions.
    fn rtree_remove_must_be_successful(
        removed: Option<BboxedIndex<GenericNode<PI, GenericIndex<CW>>>>,
    ) {
        debug_assert!(
            removed.is_some(),
            "removed node's bbox did not match any of the R-tree's envelopes.
            There are two likely reasons for this. The node may have not
            existed already. Or the node's bbox may have changed without being
            reinserted in the R-tree, making it impossible to find the node
            using an R-tree query, which is inevitably fatal"
        );
    }
}

impl<
        PW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<Index = PI> + Copy,
        DW: AccessDotWeight + Into<PW> + GetLayer,
        SW: AccessSegWeight + Into<PW> + GetLayer,
        BW: AccessBendWeight + Into<PW> + GetLayer,
        CW: Clone,
        Cel: Copy,
        PI: GetPetgraphIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + PartialEq + Copy,
        DI: GetPetgraphIndex + Into<PI> + Copy,
        SI: GetPetgraphIndex + Into<PI> + Copy,
        BI: GetPetgraphIndex + Into<PI> + Copy,
    > GeometryWithRtree<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>
{
    fn init_dot_bbox(&mut self, dot: DI) {
        self.rtree.insert(self.make_dot_bbox(dot));
    }

    fn init_seg_bbox(&mut self, seg: SI) {
        self.rtree.insert(self.make_seg_bbox(seg));
    }

    fn init_bend_bbox(&mut self, bend: BI) {
        self.rtree.insert(self.make_bend_bbox(bend));
    }

    fn make_bbox(&self, primitive: PI) -> BboxedIndex<GenericNode<PI, GenericIndex<CW>>> {
        if let Ok(dot) = <PI as TryInto<DI>>::try_into(primitive) {
            self.make_dot_bbox(dot)
        } else if let Ok(seg) = <PI as TryInto<SI>>::try_into(primitive) {
            self.make_seg_bbox(seg)
        } else if let Ok(bend) = <PI as TryInto<BI>>::try_into(primitive) {
            self.make_bend_bbox(bend)
        } else {
            unreachable!();
        }
    }

    fn make_dot_bbox(&self, dot: DI) -> BboxedIndex<GenericNode<PI, GenericIndex<CW>>> {
        BboxedIndex::new(
            Bbox::new(
                self.geometry
                    .dot_shape(dot)
                    .envelope_3d(0.0, self.layer(dot.into())),
            ),
            GenericNode::Primitive(dot.into()),
        )
    }

    fn make_seg_bbox(&self, seg: SI) -> BboxedIndex<GenericNode<PI, GenericIndex<CW>>> {
        BboxedIndex::new(
            Bbox::new(
                self.geometry
                    .seg_shape(seg)
                    .envelope_3d(0.0, self.layer(seg.into())),
            ),
            GenericNode::Primitive(seg.into()),
        )
    }

    fn make_bend_bbox(&self, bend: BI) -> BboxedIndex<GenericNode<PI, GenericIndex<CW>>> {
        BboxedIndex::new(
            Bbox::new(
                self.geometry
                    .bend_shape(bend)
                    .envelope_3d(0.0, self.layer(bend.into())),
            ),
            GenericNode::Primitive(bend.into()),
        )
    }

    fn make_compound_bbox(
        &self,
        compound: GenericIndex<CW>,
    ) -> BboxedIndex<GenericNode<PI, GenericIndex<CW>>> {
        let mut aabb = AABB::<[f64; 3]>::new_empty();

        // NOTE(fogti): perhaps allow `entry_label` to specify if it
        // should be considered part of the bounding box or not
        for (_, member) in self.geometry.compound_members(compound) {
            aabb.merge(&self.make_bbox(member).geom().aabb);
        }

        BboxedIndex::new(Bbox::new(aabb), GenericNode::Compound(compound))
    }

    fn shape(&self, primitive: PI) -> PrimitiveShape {
        if let Ok(dot) = <PI as TryInto<DI>>::try_into(primitive) {
            self.geometry.dot_shape(dot)
        } else if let Ok(seg) = <PI as TryInto<SI>>::try_into(primitive) {
            self.geometry.seg_shape(seg)
        } else if let Ok(bend) = <PI as TryInto<BI>>::try_into(primitive) {
            self.geometry.bend_shape(bend)
        } else {
            unreachable!();
        }
    }

    fn layer(&self, primitive: PI) -> usize {
        if let Ok(dot) = <PI as TryInto<DI>>::try_into(primitive) {
            self.geometry.dot_weight(dot).layer()
        } else if let Ok(seg) = <PI as TryInto<SI>>::try_into(primitive) {
            self.geometry.seg_weight(seg).layer()
        } else if let Ok(bend) = <PI as TryInto<BI>>::try_into(primitive) {
            self.geometry.bend_weight(bend).layer()
        } else {
            unreachable!();
        }
    }

    fn test_envelopes(&self) -> bool {
        !self.rtree.iter().any(|wrapper| {
            // TODO: Test envelopes of compounds too.
            let GenericNode::Primitive(primitive_node) = wrapper.data else {
                return false;
            };
            let shape = self.shape(primitive_node);
            let layer = self.layer(primitive_node);
            let wrapper = BboxedIndex::new(
                Bbox::new(shape.envelope_3d(0.0, layer)),
                GenericNode::Primitive(primitive_node),
            );
            !self
                .rtree
                .locate_in_envelope(&shape.envelope_3d(0.0, layer))
                .any(|w| *w == wrapper)
        })
    }
}

impl<
        PW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<Index = PI> + Copy,
        DW: AccessDotWeight + Into<PW> + GetLayer,
        SW: AccessSegWeight + Into<PW> + GetLayer,
        BW: AccessBendWeight + Into<PW> + GetLayer,
        CW: Clone,
        Cel: Copy,
        PI: GetPetgraphIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + PartialEq + Copy,
        DI: GetPetgraphIndex + Into<PI> + Copy,
        SI: GetPetgraphIndex + Into<PI> + Copy,
        BI: GetPetgraphIndex + Into<PI> + Copy,
    > ManageCompounds<CW> for GeometryWithRtree<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>
{
    type GeneralIndex = PI;
    type EntryLabel = Cel;

    fn add_compound(&mut self, weight: CW) -> GenericIndex<CW> {
        let compound = self.geometry.add_compound(weight);
        self.rtree.insert(self.make_compound_bbox(compound));
        compound
    }

    fn remove_compound(&mut self, compound: GenericIndex<CW>) {
        Self::rtree_remove_must_be_successful(
            self.rtree.remove(&self.make_compound_bbox(compound)),
        );
        self.geometry.remove_compound(compound);
    }

    fn add_to_compound<I>(&mut self, primitive: I, label: Cel, compound: GenericIndex<CW>)
    where
        I: Copy + GetPetgraphIndex,
    {
        self.geometry.add_to_compound(primitive, label, compound);
    }

    fn compound_weight(&self, compound: GenericIndex<CW>) -> &CW {
        self.geometry.compound_weight(compound)
    }

    fn compound_members(
        &self,
        compound: GenericIndex<CW>,
    ) -> impl Iterator<Item = (Cel, Self::GeneralIndex)> {
        self.geometry.compound_members(compound)
    }

    fn compounds<I>(&self, node: I) -> impl Iterator<Item = (Cel, GenericIndex<CW>)>
    where
        I: Copy + GetPetgraphIndex,
    {
        self.geometry.compounds(node)
    }
}

impl<
        PW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<Index = PI> + Copy,
        DW: AccessDotWeight + Into<PW> + GetLayer,
        SW: AccessSegWeight + Into<PW> + GetLayer,
        BW: AccessBendWeight + Into<PW> + GetLayer,
        CW: Clone,
        Cel: Copy,
        PI: GetPetgraphIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Eq + Ord + Copy,
        DI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
        SI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
        BI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
    > ApplyGeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>
    for GeometryWithRtree<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>
{
    fn apply(&mut self, edit: &GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>) {
        // First we remove every node that is in the edit. But we have to do
        // this in a correct order, because otherwise some removals of some
        // nodes may fail due to inadvertent invalidation of the bboxes of these
        // nodes.
        //
        // We chose to first remove compounds, then bends and segs, and only
        // then dots.

        for (compound, (maybe_old_data, ..)) in &edit.compounds {
            if maybe_old_data.is_some() {
                self.remove_compound(*compound);
            }
        }

        // Because removal of a bend will invalidate bboxes of the bends wrapped
        // around it, we first remove all the edited bends from the R-tree, and
        // their nodes from the graph only afterwards.
        for (bend, (maybe_old_data, ..)) in &edit.bends {
            if maybe_old_data.is_some() {
                Self::rtree_remove_must_be_successful(
                    self.rtree.remove(&self.make_bend_bbox(*bend)),
                );
            }
        }

        for (bend, (maybe_old_data, ..)) in &edit.bends {
            if maybe_old_data.is_some() {
                self.geometry.remove_primitive((*bend).into());
            }
        }

        for (seg, (maybe_old_data, ..)) in &edit.segs {
            if maybe_old_data.is_some() {
                self.remove_seg(*seg);
            }
        }

        for (dot, (maybe_old_data, maybe_new_data)) in &edit.dots {
            if let (Some(_), None) = (maybe_old_data, maybe_new_data) {
                self.remove_dot(*dot);
            }
        }

        // We have special handling for modified (but not removed or created)
        // dots because changing a dot also changes its limbs, which however may
        // not be in the edit.
        //
        // XXX: It may be possible to merge all these three for loops over dots.
        for (dot, (maybe_old_data, maybe_new_data)) in &edit.dots {
            if let (Some(_), Some(weight)) = (maybe_old_data, maybe_new_data) {
                self.modify_dot(*dot, |geometry, dot| {
                    // Note that we do not remove the dot. This is because doing
                    // so removes its edges, which we would have to restore
                    // afterwards. So it's easier to only update the weight.

                    // Despite this method's name, it actually does not add the
                    // dot, it updates it.
                    geometry
                        .add_dot_at_index(GenericIndex::<DW>::new(dot.petgraph_index()), *weight);
                })
            }
        }

        // Now we add every node that is created or modified (but not removed)
        // in the edit. This also has to be done in a right order, which we
        // chose to be exactly the opposite of the order of the removal which we
        // just did.

        for (dot, (maybe_old_data, maybe_new_data)) in &edit.dots {
            if let (None, Some(weight)) = (maybe_old_data, maybe_new_data) {
                self.add_dot_at_index(*dot, *weight);
            }
        }

        for (seg, (.., maybe_new_data)) in &edit.segs {
            if let Some(((from, to), weight)) = maybe_new_data {
                self.add_seg_at_index(*seg, *from, *to, *weight);
            }
        }

        // Just as with removal, we handle bend layout nodes and their bboxes
        // separately to prevent failures from inadvertent bbox invalidation.

        for (bend, (.., maybe_new_data)) in &edit.bends {
            if let Some(((from, to, core, ..), weight)) = maybe_new_data {
                self.geometry.add_bend_at_index(
                    GenericIndex::<BW>::new(bend.petgraph_index()),
                    *from,
                    *to,
                    *core,
                    *weight,
                );
            }
        }

        // We attach bends to other bends only after all bends have been
        // created, since we cannot guarantee that their inner bends will exist,
        // and attaching to an inexistent bend will result in a crash.
        for (bend, (.., maybe_new_data)) in &edit.bends {
            if let Some(((.., maybe_inner), ..)) = maybe_new_data {
                self.geometry.reattach_bend(*bend, *maybe_inner);
            }
        }

        for (bend, (.., maybe_new_data)) in &edit.bends {
            if let Some(..) = maybe_new_data {
                self.init_bend_bbox(*bend);
            }
        }

        for (compound, (.., maybe_new_data)) in &edit.compounds {
            if let Some((members, weight)) = maybe_new_data {
                self.add_compound_at_index(*compound, weight.clone());

                for (entry_label, member) in members {
                    self.geometry.add_to_compound(
                        GenericIndex::<PW>::new(member.petgraph_index()),
                        *entry_label,
                        *compound,
                    );
                }
            }
        }
    }
}
