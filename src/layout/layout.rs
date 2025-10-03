// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use contracts_try::debug_ensures;
use derive_getters::Getters;
use enum_dispatch::enum_dispatch;
use geo::Point;
use rstar::AABB;

use crate::{
    drawing::{
        band::BandTermsegIndex,
        bend::{BendIndex, BendWeight, LooseBendWeight},
        dot::{
            DotIndex, DotWeight, FixedDotIndex, FixedDotWeight, GeneralDotWeight, LooseDotIndex,
            LooseDotWeight,
        },
        gear::GearIndex,
        graph::{GetMaybeNet, IsInLayer, MakePrimitiveRef, PrimitiveIndex, PrimitiveWeight},
        primitive::{GetLimbs, MakePrimitiveShape},
        rules::AccessRules,
        seg::{
            FixedSegIndex, FixedSegWeight, LoneLooseSegIndex, LoneLooseSegWeight, SegIndex,
            SegWeight, SeqLooseSegIndex, SeqLooseSegWeight,
        },
        Cane, Drawing, DrawingEdit, DrawingException, Infringement,
    },
    geometry::{
        compound::ManageCompounds,
        edit::ApplyGeometryEdit,
        shape::{AccessShape, Shape},
        GenericNode, GetLayer, GetSetPos,
    },
    graph::{GenericIndex, GetIndex, MakeRef},
    layout::{
        poly::{add_poly_with_nodes_intern, MakePolygon, PolyWeight},
        via::{Via, ViaWeight},
    },
    math::RotationSense,
};

/// Represents a weight for various compounds
#[derive(Clone, Copy, Debug)]
#[enum_dispatch(GetMaybeNet, IsInLayer)]
pub enum CompoundWeight {
    /// Represents the weight of a polygon compound, includes its basic [`Layout`] information
    Poly(PolyWeight),
    /// Represents Via weight properties, containing its [`Layout`] properties
    Via(ViaWeight),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompoundEntryLabel {
    Normal,
    Apex,
    Fillet,
}

/// The alias to differ node types
pub type NodeIndex = GenericNode<PrimitiveIndex, GenericIndex<CompoundWeight>>;
pub type LayoutEdit = DrawingEdit<CompoundWeight, CompoundEntryLabel>;

#[derive(Clone, Debug, Getters)]
/// Structure for managing the Layout design
pub struct Layout<R> {
    pub(super) drawing: Drawing<CompoundWeight, CompoundEntryLabel, R>,
}

impl<R> Layout<R> {
    pub fn new(drawing: Drawing<CompoundWeight, CompoundEntryLabel, R>) -> Self {
        Self { drawing }
    }
}

impl<R: AccessRules> Layout<R> {
    pub fn add_cane(
        &mut self,
        recorder: &mut LayoutEdit,
        from: DotIndex,
        around: GearIndex,
        dot_weight: LooseDotWeight,
        seg_weight: SeqLooseSegWeight,
        bend_weight: LooseBendWeight,
        sense: RotationSense,
    ) -> Result<Cane, DrawingException> {
        self.drawing.add_cane(
            recorder,
            from,
            around,
            dot_weight,
            seg_weight,
            bend_weight,
            sense,
            &|drawing, _infringer, infringee| {
                // Don't infringe upon limbs of wraparound's filleteds.
                !drawing
                    .overlapees(around.into())
                    .find(|overlapee| {
                        PrimitiveIndex::from(overlapee.2)
                            .primitive_ref(drawing)
                            .limbs()
                            .contains(&infringee)
                    })
                    .is_some()
            },
        )
    }

    pub fn insert_cane(
        &mut self,
        recorder: &mut LayoutEdit,
        from: DotIndex,
        around: GearIndex,
        dot_weight: LooseDotWeight,
        seg_weight: SeqLooseSegWeight,
        bend_weight: LooseBendWeight,
        sense: RotationSense,
    ) -> Result<Cane, DrawingException> {
        self.drawing.insert_cane(
            recorder,
            from,
            around,
            dot_weight,
            seg_weight,
            bend_weight,
            sense,
            &|_drawing, _infringer, _infringee| true,
        )
    }

    pub fn remove_cane(&mut self, recorder: &mut LayoutEdit, cane: &Cane, face: LooseDotIndex) {
        self.drawing.remove_cane(recorder, cane, face)
    }

    pub fn remove_termseg(&mut self, recorder: &mut LayoutEdit, termseg: BandTermsegIndex) {
        self.drawing.remove_termseg(recorder, termseg)
    }

    #[debug_ensures(ret.is_ok() -> self.drawing.node_count() == old(self.drawing.node_count()) + weight.to_layer - weight.from_layer + 2)]
    #[debug_ensures(ret.is_err() -> self.drawing.node_count() == old(self.drawing.node_count()))]
    /// Insert [`Via`] into the [`Layout`]
    pub fn add_via(
        &mut self,
        recorder: &mut LayoutEdit,
        weight: ViaWeight,
    ) -> Result<(GenericIndex<ViaWeight>, Vec<FixedDotIndex>), Infringement> {
        let compound = self.drawing.add_compound(recorder, weight.into());
        let mut dots = vec![];

        for layer in weight.from_layer..=weight.to_layer {
            match self.drawing.add_fixed_dot(
                recorder,
                FixedDotWeight(GeneralDotWeight {
                    circle: weight.circle,
                    layer,
                    maybe_net: weight.maybe_net,
                }),
            ) {
                Ok(dot) => {
                    self.drawing.add_to_compound(
                        recorder,
                        dot,
                        CompoundEntryLabel::Normal,
                        compound,
                    );
                    dots.push(dot);
                }
                Err(err) => {
                    // Remove inserted dots.
                    self.drawing.remove_compound(recorder, compound);

                    for dot in dots.iter().rev() {
                        self.drawing.remove_fixed_dot(recorder, *dot);
                    }

                    return Err(err);
                }
            }
        }

        Ok((GenericIndex::<ViaWeight>::new(compound.index()), dots))
    }

    pub fn add_fixed_dot(
        &mut self,
        recorder: &mut LayoutEdit,
        weight: FixedDotWeight,
    ) -> Result<FixedDotIndex, Infringement> {
        self.drawing.add_fixed_dot(recorder, weight)
    }

    pub fn add_fixed_dot_infringably(
        &mut self,
        recorder: &mut LayoutEdit,
        weight: FixedDotWeight,
    ) -> FixedDotIndex {
        self.drawing.add_fixed_dot_infringably(recorder, weight)
    }

    pub fn add_fixed_seg(
        &mut self,
        recorder: &mut LayoutEdit,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
    ) -> Result<FixedSegIndex, DrawingException> {
        self.drawing.add_fixed_seg(recorder, from, to, weight)
    }

    pub fn add_fixed_seg_infringably(
        &mut self,
        recorder: &mut LayoutEdit,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
    ) -> FixedSegIndex {
        self.drawing
            .add_fixed_seg_infringably(recorder, from, to, weight)
    }

    pub fn add_lone_loose_seg(
        &mut self,
        recorder: &mut LayoutEdit,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: LoneLooseSegWeight,
    ) -> Result<LoneLooseSegIndex, DrawingException> {
        self.drawing.add_lone_loose_seg(recorder, from, to, weight)
    }

    pub fn add_seq_loose_seg(
        &mut self,
        recorder: &mut LayoutEdit,
        from: DotIndex,
        to: LooseDotIndex,
        weight: SeqLooseSegWeight,
    ) -> Result<SeqLooseSegIndex, DrawingException> {
        self.drawing.add_seq_loose_seg(recorder, from, to, weight)
    }

    pub fn move_dot(
        &mut self,
        recorder: &mut LayoutEdit,
        dot: DotIndex,
        to: Point,
    ) -> Result<(), Infringement> {
        self.drawing.move_dot(recorder, dot, to)
    }

    pub fn add_poly(
        &mut self,
        recorder: &mut LayoutEdit,
        weight: PolyWeight,
    ) -> GenericIndex<PolyWeight> {
        GenericIndex::<PolyWeight>::new(
            self.drawing
                .add_compound(recorder, CompoundWeight::Poly(weight))
                .index(),
        )
    }

    /// insert a polygon based upon the border nodes, and computes + returns the
    /// associated apex
    pub fn add_poly_with_nodes(
        &mut self,
        recorder: &mut LayoutEdit,
        weight: PolyWeight,
        nodes: &[PrimitiveIndex],
        fillets: &[FixedDotIndex],
    ) -> (GenericIndex<PolyWeight>, FixedDotIndex) {
        let layer = weight.layer();
        let maybe_net = weight.maybe_net();
        let poly = self.add_poly(recorder, weight);
        (
            poly,
            add_poly_with_nodes_intern(self, recorder, poly, nodes, fillets, layer, maybe_net),
        )
    }

    pub fn remove_band(
        &mut self,
        recorder: &mut LayoutEdit,
        band: BandTermsegIndex,
    ) -> Result<(), DrawingException> {
        self.drawing.remove_band(recorder, band)
    }

    pub fn poly_nodes(&self) -> impl Iterator<Item = GenericIndex<PolyWeight>> + '_ {
        self.drawing.rtree().iter().filter_map(|wrapper| {
            if let NodeIndex::Compound(compound) = wrapper.data {
                if let CompoundWeight::Poly(..) = self.drawing.compound_weight(compound) {
                    return Some(GenericIndex::<PolyWeight>::new(compound.index()));
                }
            }

            None
        })
    }

    pub fn layer_poly_nodes(
        &self,
        layer: usize,
    ) -> impl Iterator<Item = GenericIndex<PolyWeight>> + '_ {
        self.drawing
            .rtree()
            .locate_in_envelope_intersecting(&AABB::from_corners(
                [-f64::INFINITY, -f64::INFINITY, layer as f64],
                [f64::INFINITY, f64::INFINITY, layer as f64],
            ))
            .filter_map(|wrapper| {
                if let NodeIndex::Compound(compound) = wrapper.data {
                    if let CompoundWeight::Poly(..) = self.drawing.compound_weight(compound) {
                        return Some(GenericIndex::<PolyWeight>::new(compound.index()));
                    }
                }

                None
            })
    }

    pub fn poly_members(
        &self,
        poly: GenericIndex<PolyWeight>,
    ) -> impl Iterator<Item = (CompoundEntryLabel, PrimitiveIndex)> + '_ {
        self.drawing
            .geometry()
            .compound_members(GenericIndex::new(poly.index()))
    }

    fn compound_shape(&self, compound: GenericIndex<CompoundWeight>) -> Shape {
        match self.drawing.compound_weight(compound) {
            CompoundWeight::Poly(_) => GenericIndex::<PolyWeight>::new(compound.index())
                .ref_(self)
                .shape()
                .into(),
            CompoundWeight::Via(weight) => weight.shape().into(),
        }
    }

    pub fn node_shape(&self, index: NodeIndex) -> Shape {
        match index {
            NodeIndex::Primitive(primitive) => {
                primitive.primitive_ref(&self.drawing).shape().into()
            }
            NodeIndex::Compound(compound) => self.compound_shape(compound),
        }
    }

    pub fn primitive_poly(&self, primitive: PrimitiveIndex) -> Option<GenericIndex<PolyWeight>> {
        self.drawing()
            .compounds(GenericIndex::<()>::new(primitive.index()))
            .find_map(|(_, compound)| {
                if let CompoundWeight::Poly(_) = self.drawing().compound_weight(compound) {
                    Some(compound)
                } else {
                    None
                }
            })
            .map(|compound| GenericIndex::<PolyWeight>::new(compound.index()))
    }

    /// Checks if a node is not a primitive part of a compound, and if yes, returns its apex and center
    pub fn apex_of_compoundless_node(
        &self,
        node: NodeIndex,
        active_layer: usize,
    ) -> Option<(FixedDotIndex, Point)> {
        fn handle_fixed_dot<R: AccessRules>(
            drawing: &Drawing<CompoundWeight, CompoundEntryLabel, R>,
            index: PrimitiveIndex,
        ) -> Option<(FixedDotIndex, FixedDotWeight)> {
            let PrimitiveIndex::FixedDot(dot) = index else {
                return None;
            };
            if let PrimitiveWeight::FixedDot(weight) =
                drawing.geometry().primitive_weight(dot.index())
            {
                Some((dot, weight))
            } else {
                unreachable!()
            }
        }

        match node {
            NodeIndex::Primitive(primitive) => {
                if self
                    .drawing()
                    .geometry()
                    // TODO: Add `.compounds()` method working on `PrimitiveIndex`.
                    .compounds(GenericIndex::<()>::new(primitive.index()))
                    .next()
                    .is_some()
                {
                    return None;
                }
                handle_fixed_dot(&self.drawing, primitive).map(|(dot, weight)| (dot, weight.pos()))
            }
            NodeIndex::Compound(compound) => Some(match self.drawing.compound_weight(compound) {
                CompoundWeight::Poly(_) => {
                    let poly = GenericIndex::<PolyWeight>::new(compound.index()).ref_(self);
                    (poly.apex(), poly.shape().center())
                }
                CompoundWeight::Via(weight) => {
                    let mut dots = self.drawing.geometry().compound_members(compound);
                    let apex = loop {
                        // this returns None if the via is not present on this layer
                        let (entry_label, dot) = dots.next()?;
                        if entry_label == CompoundEntryLabel::Apex {
                            if let Some((dot, weight)) = handle_fixed_dot(&self.drawing, dot) {
                                if weight.layer() == active_layer {
                                    break dot;
                                }
                            }
                        }
                    };
                    (apex, weight.shape().center())
                }
            }),
        }
    }

    pub fn rules(&self) -> &R {
        self.drawing.rules()
    }

    pub fn rules_mut(&mut self) -> &mut R {
        self.drawing.rules_mut()
    }

    pub fn via(&self, index: GenericIndex<ViaWeight>) -> Via<'_, R> {
        Via::new(index, self.drawing())
    }
}

impl<R: AccessRules>
    ApplyGeometryEdit<
        DotWeight,
        SegWeight,
        BendWeight,
        CompoundWeight,
        CompoundEntryLabel,
        PrimitiveIndex,
        DotIndex,
        SegIndex,
        BendIndex,
    > for Layout<R>
{
    fn apply(&mut self, edit: &LayoutEdit) {
        self.drawing.apply(edit);
    }
}
