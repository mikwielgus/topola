use contracts::debug_ensures;
use enum_dispatch::enum_dispatch;
use geo::Point;
use rstar::AABB;

use crate::{
    drawing::{
        band::BandIndex,
        bend::LooseBendWeight,
        dot::{DotIndex, FixedDotIndex, FixedDotWeight, LooseDotIndex, LooseDotWeight},
        graph::{GetMaybeNet, PrimitiveIndex},
        primitive::{GetJoints, GetOtherJoint},
        rules::RulesTrait,
        seg::{
            FixedSegIndex, FixedSegWeight, LoneLooseSegIndex, LoneLooseSegWeight, SeqLooseSegIndex,
            SeqLooseSegWeight,
        },
        segbend::Segbend,
        wraparoundable::WraparoundableIndex,
        Drawing, Infringement, LayoutException,
    },
    geometry::{compound::CompoundManagerTrait, primitive::PrimitiveShapeTrait, GenericNode},
    graph::{GenericIndex, GetNodeIndex},
    layout::{
        via::{Via, ViaWeight},
        zone::{Zone, ZoneWeight},
    },
};

#[derive(Debug, Clone, Copy)]
#[enum_dispatch(GetMaybeNet)]
pub enum CompoundWeight {
    Zone(ZoneWeight),
    Via(ViaWeight),
}

pub type NodeIndex = GenericNode<PrimitiveIndex, GenericIndex<CompoundWeight>>;

#[derive(Debug)]
pub struct Layout<R: RulesTrait> {
    drawing: Drawing<CompoundWeight, R>,
}

impl<R: RulesTrait> Layout<R> {
    pub fn new(drawing: Drawing<CompoundWeight, R>) -> Self {
        Self { drawing }
    }

    pub fn remove_band(&mut self, band: BandIndex) {
        self.drawing.remove_band(band);
    }

    pub fn remove_segbend(&mut self, segbend: &Segbend, face: LooseDotIndex) {
        self.drawing.remove_segbend(segbend, face)
    }

    pub fn insert_segbend(
        &mut self,
        from: DotIndex,
        around: WraparoundableIndex,
        dot_weight: LooseDotWeight,
        seg_weight: SeqLooseSegWeight,
        bend_weight: LooseBendWeight,
        cw: bool,
    ) -> Result<Segbend, LayoutException> {
        self.drawing
            .insert_segbend(from, around, dot_weight, seg_weight, bend_weight, cw)
    }

    #[debug_ensures(ret.is_ok() -> self.drawing.node_count() == old(self.drawing.node_count()) + weight.to_layer - weight.from_layer)]
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

        Ok(GenericIndex::<ViaWeight>::new(compound.node_index()))
    }

    pub fn add_fixed_dot(&mut self, weight: FixedDotWeight) -> Result<FixedDotIndex, Infringement> {
        self.drawing.add_fixed_dot(weight)
    }

    pub fn add_fixed_dot_infringably(&mut self, weight: FixedDotWeight) -> FixedDotIndex {
        self.drawing.add_fixed_dot_infringably(weight)
    }

    pub fn add_zone_fixed_dot(
        &mut self,
        weight: FixedDotWeight,
        zone: GenericIndex<ZoneWeight>,
    ) -> Result<FixedDotIndex, Infringement> {
        let maybe_dot = self.drawing.add_fixed_dot(weight);

        if let Ok(dot) = maybe_dot {
            self.drawing.add_to_compound(dot, zone.into());
        }

        maybe_dot
    }

    pub fn add_zone_fixed_dot_infringably(
        &mut self,
        weight: FixedDotWeight,
        zone: GenericIndex<ZoneWeight>,
    ) -> FixedDotIndex {
        let dot = self.drawing.add_fixed_dot_infringably(weight);
        self.drawing.add_to_compound(dot, zone.into());
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

    pub fn add_zone_fixed_seg(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
        zone: GenericIndex<ZoneWeight>,
    ) -> Result<FixedSegIndex, Infringement> {
        let maybe_seg = self.add_fixed_seg(from, to, weight);

        if let Ok(seg) = maybe_seg {
            self.drawing.add_to_compound(seg, zone.into());
        }

        maybe_seg
    }

    pub fn add_zone_fixed_seg_infringably(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
        zone: GenericIndex<ZoneWeight>,
    ) -> FixedSegIndex {
        let seg = self.add_fixed_seg_infringably(from, to, weight);
        self.drawing.add_to_compound(seg, zone.into());
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

    pub fn add_zone(&mut self, weight: ZoneWeight) -> GenericIndex<ZoneWeight> {
        GenericIndex::<ZoneWeight>::new(
            self.drawing
                .add_compound(CompoundWeight::Zone(weight))
                .node_index(),
        )
    }

    pub fn zones<W: 'static>(
        &self,
        node: GenericIndex<W>,
    ) -> impl Iterator<Item = GenericIndex<CompoundWeight>> + '_ {
        self.drawing.compounds(node)
    }

    pub fn band_length(&self, band: BandIndex) -> f64 {
        match band {
            BandIndex::Straight(seg) => self.drawing.geometry().seg_shape(seg.into()).length(),
            BandIndex::Bended(start_seg) => {
                let mut length = self.drawing.geometry().seg_shape(start_seg.into()).length();
                let start_dot = self.drawing.primitive(start_seg).joints().1;

                let bend = self.drawing.primitive(start_dot).bend();
                length += self.drawing.geometry().bend_shape(bend.into()).length();

                let mut prev_dot = self.drawing.primitive(bend).other_joint(start_dot.into());
                let mut seg = self.drawing.primitive(prev_dot).seg().unwrap();
                length += self.drawing.geometry().seg_shape(seg.into()).length();

                while let DotIndex::Loose(dot) =
                    self.drawing.primitive(seg).other_joint(prev_dot.into())
                {
                    let bend = self.drawing.primitive(dot).bend();
                    length += self.drawing.geometry().bend_shape(bend.into()).length();

                    prev_dot = self.drawing.primitive(bend).other_joint(dot);
                    seg = self.drawing.primitive(prev_dot).seg().unwrap();
                    length += self.drawing.geometry().seg_shape(seg.into()).length();
                }

                length
            }
        }
    }

    pub fn zone_nodes(&self) -> impl Iterator<Item = GenericIndex<ZoneWeight>> + '_ {
        self.drawing.rtree().iter().filter_map(|wrapper| {
            if let NodeIndex::Compound(compound) = wrapper.data {
                if let CompoundWeight::Zone(..) = self.drawing.compound_weight(compound) {
                    return Some(GenericIndex::<ZoneWeight>::new(compound.node_index()));
                }
            }

            None
        })
    }

    pub fn layer_zone_nodes(
        &self,
        layer: usize,
    ) -> impl Iterator<Item = GenericIndex<ZoneWeight>> + '_ {
        self.drawing
            .rtree()
            .locate_in_envelope_intersecting(&AABB::from_corners(
                [-f64::INFINITY, -f64::INFINITY, layer as f64],
                [f64::INFINITY, f64::INFINITY, layer as f64],
            ))
            .filter_map(|wrapper| {
                if let NodeIndex::Compound(compound) = wrapper.data {
                    if let CompoundWeight::Zone(..) = self.drawing.compound_weight(compound) {
                        return Some(GenericIndex::<ZoneWeight>::new(compound.node_index()));
                    }
                }

                None
            })
    }

    pub fn zone_members(
        &self,
        zone: GenericIndex<ZoneWeight>,
    ) -> impl Iterator<Item = PrimitiveIndex> + '_ {
        self.drawing
            .geometry()
            .compound_members(GenericIndex::new(zone.node_index()))
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

    pub fn zone(&self, index: GenericIndex<ZoneWeight>) -> Zone<R> {
        Zone::new(index, self)
    }

    pub fn via(&self, index: GenericIndex<ViaWeight>) -> Via<R> {
        Via::new(index, self)
    }
}
