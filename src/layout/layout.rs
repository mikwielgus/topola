use contracts::debug_ensures;
use enum_dispatch::enum_dispatch;
use geo::Point;
use rstar::AABB;

use crate::{
    drawing::{
        band::BandFirstSegIndex,
        bend::LooseBendWeight,
        cane::Cane,
        dot::{DotIndex, FixedDotIndex, FixedDotWeight, LooseDotIndex, LooseDotWeight},
        graph::{GetMaybeNet, PrimitiveIndex},
        primitive::{GetJoints, GetOtherJoint},
        rules::AccessRules,
        seg::{
            FixedSegIndex, FixedSegWeight, LoneLooseSegIndex, LoneLooseSegWeight, SeqLooseSegIndex,
            SeqLooseSegWeight,
        },
        wraparoundable::WraparoundableIndex,
        Drawing, Infringement, LayoutException,
    },
    geometry::{compound::ManageCompounds, shape::MeasureLength, GenericNode},
    graph::{GenericIndex, GetPetgraphIndex},
    layout::{
        poly::{Poly, PolyWeight},
        via::{Via, ViaWeight},
    },
};

#[derive(Debug, Clone, Copy)]
#[enum_dispatch(GetMaybeNet)]
pub enum CompoundWeight {
    Poly(PolyWeight),
    Via(ViaWeight),
}

pub type NodeIndex = GenericNode<PrimitiveIndex, GenericIndex<CompoundWeight>>;

#[derive(Debug)]
pub struct Layout<R: AccessRules> {
    drawing: Drawing<CompoundWeight, R>,
}

impl<R: AccessRules> Layout<R> {
    pub fn new(drawing: Drawing<CompoundWeight, R>) -> Self {
        Self { drawing }
    }

    pub fn insert_cane(
        &mut self,
        from: DotIndex,
        around: WraparoundableIndex,
        dot_weight: LooseDotWeight,
        seg_weight: SeqLooseSegWeight,
        bend_weight: LooseBendWeight,
        cw: bool,
    ) -> Result<Cane, LayoutException> {
        self.drawing
            .insert_cane(from, around, dot_weight, seg_weight, bend_weight, cw)
    }

    pub fn remove_cane(&mut self, cane: &Cane, face: LooseDotIndex) {
        self.drawing.remove_cane(cane, face)
    }

    #[debug_ensures(ret.is_ok() -> self.drawing.node_count() == old(self.drawing.node_count()) + weight.to_layer - weight.from_layer + 2)]
    #[debug_ensures(ret.is_err() -> self.drawing.node_count() == old(self.drawing.node_count()))]
    pub fn add_via(&mut self, weight: ViaWeight) -> Result<GenericIndex<ViaWeight>, Infringement> {
        let compound = self.drawing.add_compound(weight.into());
        let mut dots = vec![];

        for layer in weight.from_layer..=weight.to_layer {
            match self.drawing.add_fixed_dot(FixedDotWeight {
                circle: weight.circle,
                layer,
                maybe_net: weight.maybe_net,
            }) {
                Ok(dot) => {
                    self.drawing.add_to_compound(dot, compound);
                    dots.push(dot);
                }
                Err(err) => {
                    // Remove inserted dots.

                    self.drawing.remove_compound(compound);

                    for dot in dots.iter().rev() {
                        self.drawing.remove_fixed_dot(*dot);
                    }

                    return Err(err);
                }
            }
        }

        Ok(GenericIndex::<ViaWeight>::new(compound.petgraph_index()))
    }

    pub fn add_fixed_dot(&mut self, weight: FixedDotWeight) -> Result<FixedDotIndex, Infringement> {
        self.drawing.add_fixed_dot(weight)
    }

    pub fn add_fixed_dot_infringably(&mut self, weight: FixedDotWeight) -> FixedDotIndex {
        self.drawing.add_fixed_dot_infringably(weight)
    }

    pub fn add_poly_fixed_dot(
        &mut self,
        weight: FixedDotWeight,
        poly: GenericIndex<PolyWeight>,
    ) -> Result<FixedDotIndex, Infringement> {
        let maybe_dot = self.drawing.add_fixed_dot(weight);

        if let Ok(dot) = maybe_dot {
            self.drawing.add_to_compound(dot, poly.into());
        }

        maybe_dot
    }

    pub fn add_poly_fixed_dot_infringably(
        &mut self,
        weight: FixedDotWeight,
        poly: GenericIndex<PolyWeight>,
    ) -> FixedDotIndex {
        let dot = self.drawing.add_fixed_dot_infringably(weight);
        self.drawing.add_to_compound(dot, poly.into());
        dot
    }

    pub fn add_fixed_seg(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
    ) -> Result<FixedSegIndex, Infringement> {
        self.drawing.add_fixed_seg(from, to, weight)
    }

    pub fn add_fixed_seg_infringably(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
    ) -> FixedSegIndex {
        self.drawing.add_fixed_seg_infringably(from, to, weight)
    }

    pub fn add_poly_fixed_seg(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
        poly: GenericIndex<PolyWeight>,
    ) -> Result<FixedSegIndex, Infringement> {
        let maybe_seg = self.add_fixed_seg(from, to, weight);

        if let Ok(seg) = maybe_seg {
            self.drawing.add_to_compound(seg, poly.into());
        }

        maybe_seg
    }

    pub fn add_poly_fixed_seg_infringably(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
        poly: GenericIndex<PolyWeight>,
    ) -> FixedSegIndex {
        let seg = self.add_fixed_seg_infringably(from, to, weight);
        self.drawing.add_to_compound(seg, poly.into());
        seg
    }

    pub fn add_lone_loose_seg(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: LoneLooseSegWeight,
    ) -> Result<LoneLooseSegIndex, Infringement> {
        self.drawing.add_lone_loose_seg(from, to, weight)
    }

    pub fn add_seq_loose_seg(
        &mut self,
        from: DotIndex,
        to: LooseDotIndex,
        weight: SeqLooseSegWeight,
    ) -> Result<SeqLooseSegIndex, Infringement> {
        self.drawing.add_seq_loose_seg(from, to, weight)
    }

    pub fn move_dot(&mut self, dot: DotIndex, to: Point) -> Result<(), Infringement> {
        self.drawing.move_dot(dot, to)
    }

    pub fn add_poly(&mut self, weight: PolyWeight) -> GenericIndex<PolyWeight> {
        GenericIndex::<PolyWeight>::new(
            self.drawing
                .add_compound(CompoundWeight::Poly(weight))
                .petgraph_index(),
        )
    }

    pub fn remove_band(&mut self, band: BandFirstSegIndex) {
        self.drawing.remove_band(band);
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

    pub fn drawing(&self) -> &Drawing<CompoundWeight, R> {
        &self.drawing
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
