use contracts_try::debug_ensures;
use derive_getters::Getters;
use enum_dispatch::enum_dispatch;
use geo::Point;
use rstar::AABB;

use crate::{
    drawing::{
        band::BandTermsegIndex,
        bend::{BendIndex, BendWeight, LooseBendWeight},
        cane::Cane,
        dot::{DotIndex, DotWeight, FixedDotIndex, FixedDotWeight, LooseDotIndex, LooseDotWeight},
        gear::GearIndex,
        graph::{GetMaybeNet, PrimitiveIndex, PrimitiveWeight},
        rules::AccessRules,
        seg::{
            FixedSegIndex, FixedSegWeight, LoneLooseSegIndex, LoneLooseSegWeight, SegIndex,
            SegWeight, SeqLooseSegIndex, SeqLooseSegWeight,
        },
        Drawing, DrawingEdit, DrawingException, Infringement,
    },
    geometry::{edit::ApplyGeometryEdit, GenericNode},
    graph::{GenericIndex, GetPetgraphIndex},
    layout::{
        poly::{Poly, PolyWeight},
        via::{Via, ViaWeight},
    },
};

/// Represents a weight for various compounds
#[derive(Debug, Clone, Copy)]
#[enum_dispatch(GetMaybeNet)]
pub enum CompoundWeight {
    /// Represents the weight of a polygon compound, includes its basic [`Layout`] information
    Poly(PolyWeight),
    /// Represents Via weight properties, containing its [`Layout`] properties
    Via(ViaWeight),
}

/// The alias to differ node types
pub type NodeIndex = GenericNode<PrimitiveIndex, GenericIndex<CompoundWeight>>;
pub type LayoutEdit = DrawingEdit<CompoundWeight>;

#[derive(Debug, Getters)]
/// Structure for managing the Layout design
pub struct Layout<R: AccessRules> {
    drawing: Drawing<CompoundWeight, R>,
}

impl<R: AccessRules> Layout<R> {
    pub fn new(drawing: Drawing<CompoundWeight, R>) -> Self {
        Self { drawing }
    }

    /// Insert [`Cane`] object into the [`Layout`]
    pub fn insert_cane(
        &mut self,
        recorder: &mut LayoutEdit,
        from: DotIndex,
        around: GearIndex,
        dot_weight: LooseDotWeight,
        seg_weight: SeqLooseSegWeight,
        bend_weight: LooseBendWeight,
        cw: bool,
    ) -> Result<Cane, DrawingException> {
        self.drawing.insert_cane(
            recorder,
            from,
            around,
            dot_weight,
            seg_weight,
            bend_weight,
            cw,
        )
    }

    /// Remove [`Cane`] object from the [`Layout`]
    pub fn remove_cane(&mut self, recorder: &mut LayoutEdit, cane: &Cane, face: LooseDotIndex) {
        self.drawing.remove_cane(recorder, cane, face)
    }

    #[debug_ensures(ret.is_ok() -> self.drawing.node_count() == old(self.drawing.node_count()) + weight.to_layer - weight.from_layer + 2)]
    #[debug_ensures(ret.is_err() -> self.drawing.node_count() == old(self.drawing.node_count()))]
    /// Insert [`Via`] into the [`Layout`]
    pub fn add_via(
        &mut self,
        recorder: &mut LayoutEdit,
        weight: ViaWeight,
    ) -> Result<GenericIndex<ViaWeight>, Infringement> {
        let compound = self.drawing.add_compound(recorder, weight.into());
        let mut dots = vec![];

        for layer in weight.from_layer..=weight.to_layer {
            match self.drawing.add_fixed_dot(
                recorder,
                FixedDotWeight {
                    circle: weight.circle,
                    layer,
                    maybe_net: weight.maybe_net,
                },
            ) {
                Ok(dot) => {
                    self.drawing.add_to_compound(recorder, dot, compound);
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

        Ok(GenericIndex::<ViaWeight>::new(compound.petgraph_index()))
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

    pub fn add_poly_fixed_dot(
        &mut self,
        recorder: &mut LayoutEdit,
        weight: FixedDotWeight,
        poly: GenericIndex<PolyWeight>,
    ) -> Result<FixedDotIndex, Infringement> {
        let maybe_dot = self.drawing.add_fixed_dot(recorder, weight);

        if let Ok(dot) = maybe_dot {
            self.drawing.add_to_compound(recorder, dot, poly.into());
        }

        maybe_dot
    }

    pub fn add_poly_fixed_dot_infringably(
        &mut self,
        recorder: &mut LayoutEdit,
        weight: FixedDotWeight,
        poly: GenericIndex<PolyWeight>,
    ) -> FixedDotIndex {
        let dot = self.drawing.add_fixed_dot_infringably(recorder, weight);
        self.drawing.add_to_compound(recorder, dot, poly.into());
        dot
    }

    pub fn add_fixed_seg(
        &mut self,
        recorder: &mut LayoutEdit,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
    ) -> Result<FixedSegIndex, Infringement> {
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

    pub fn add_poly_fixed_seg(
        &mut self,
        recorder: &mut LayoutEdit,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
        poly: GenericIndex<PolyWeight>,
    ) -> Result<FixedSegIndex, Infringement> {
        let maybe_seg = self.add_fixed_seg(recorder, from, to, weight);

        if let Ok(seg) = maybe_seg {
            self.drawing.add_to_compound(recorder, seg, poly.into());
        }

        maybe_seg
    }

    pub fn add_poly_fixed_seg_infringably(
        &mut self,
        recorder: &mut LayoutEdit,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
        poly: GenericIndex<PolyWeight>,
    ) -> FixedSegIndex {
        let seg = self.add_fixed_seg_infringably(recorder, from, to, weight);
        self.drawing.add_to_compound(recorder, seg, poly.into());
        seg
    }

    pub fn add_lone_loose_seg(
        &mut self,
        recorder: &mut LayoutEdit,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: LoneLooseSegWeight,
    ) -> Result<LoneLooseSegIndex, Infringement> {
        self.drawing.add_lone_loose_seg(recorder, from, to, weight)
    }

    pub fn add_seq_loose_seg(
        &mut self,
        recorder: &mut LayoutEdit,
        from: DotIndex,
        to: LooseDotIndex,
        weight: SeqLooseSegWeight,
    ) -> Result<SeqLooseSegIndex, Infringement> {
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
                .petgraph_index(),
        )
    }

    pub fn remove_band(
        &mut self,
        recorder: &mut LayoutEdit,
        band: BandTermsegIndex,
    ) -> Result<(), DrawingException> {
        self.drawing.remove_band(recorder, band)
    }

    pub fn polys<W: 'static>(
        &self,
        node: GenericIndex<W>,
    ) -> impl Iterator<Item = GenericIndex<CompoundWeight>> + '_ {
        self.drawing.compounds(node)
    }

    pub fn poly_nodes(&self) -> impl Iterator<Item = GenericIndex<PolyWeight>> + '_ {
        self.drawing.rtree().iter().filter_map(|wrapper| {
            if let NodeIndex::Compound(compound) = wrapper.data {
                if let CompoundWeight::Poly(..) = self.drawing.compound_weight(compound) {
                    return Some(GenericIndex::<PolyWeight>::new(compound.petgraph_index()));
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
                        return Some(GenericIndex::<PolyWeight>::new(compound.petgraph_index()));
                    }
                }

                None
            })
    }

    pub fn poly_members(
        &self,
        poly: GenericIndex<PolyWeight>,
    ) -> impl Iterator<Item = PrimitiveIndex> + '_ {
        self.drawing
            .geometry()
            .compound_members(GenericIndex::new(poly.petgraph_index()))
    }

    pub fn rules(&self) -> &R {
        self.drawing.rules()
    }

    pub fn rules_mut(&mut self) -> &mut R {
        self.drawing.rules_mut()
    }

    pub fn poly(&self, index: GenericIndex<PolyWeight>) -> Poly<R> {
        Poly::new(index, self)
    }

    pub fn via(&self, index: GenericIndex<ViaWeight>) -> Via<R> {
        Via::new(index, self)
    }
}

impl<R: AccessRules>
    ApplyGeometryEdit<
        PrimitiveWeight,
        DotWeight,
        SegWeight,
        BendWeight,
        CompoundWeight,
        PrimitiveIndex,
        DotIndex,
        SegIndex,
        BendIndex,
    > for Layout<R>
{
    fn apply(&mut self, edit: LayoutEdit) {
        self.drawing.apply(edit);
    }
}
