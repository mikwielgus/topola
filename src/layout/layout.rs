use geo::Point;
use petgraph::stable_graph::StableDiGraph;
use rstar::AABB;

use crate::{
    drawing::{
        bend::LooseBendWeight,
        dot::{DotIndex, FixedDotIndex, FixedDotWeight, LooseDotIndex, LooseDotWeight},
        graph::{GetLayer, PrimitiveIndex, PrimitiveWeight, Retag},
        rules::RulesTrait,
        seg::{
            FixedSegIndex, FixedSegWeight, LoneLooseSegIndex, LoneLooseSegWeight, SeqLooseSegIndex,
            SeqLooseSegWeight,
        },
        segbend::Segbend,
        wraparoundable::WraparoundableIndex,
        Drawing, Infringement, LayoutException,
    },
    geometry::{
        compound::CompoundManagerTrait, poly::PolyShape, shape::ShapeTrait, BendWeightTrait,
        DotWeightTrait, GenericNode, Geometry, GeometryLabel, GetWidth, SegWeightTrait,
    },
    graph::{GenericIndex, GetNodeIndex},
    layout::{
        connectivity::{
            BandIndex, BandWeight, ConnectivityLabel, ConnectivityWeight, ContinentIndex,
        },
        zone::{GetMaybeApex, MakePolyShape, PourZoneIndex, SolidZoneIndex, Zone, ZoneWeight},
    },
    math::Circle,
};

pub type NodeIndex = GenericNode<PrimitiveIndex, GenericIndex<ZoneWeight>>;

#[derive(Debug)]
pub struct Layout<R: RulesTrait> {
    drawing: Drawing<ZoneWeight, R>,
    connectivity: StableDiGraph<ConnectivityWeight, ConnectivityLabel, usize>,
}

impl<R: RulesTrait> Layout<R> {
    pub fn new(drawing: Drawing<ZoneWeight, R>) -> Self {
        Self {
            drawing,
            connectivity: StableDiGraph::default(),
        }
    }

    pub fn remove_band(&mut self, band: BandIndex) {
        todo!()
    }

    pub fn remove_segbend(&mut self, segbend: &Segbend, face: LooseDotIndex) {
        self.drawing.remove_segbend(segbend, face)
    }

    pub fn start_band(&mut self, from: FixedDotIndex) -> BandIndex {
        let band = self
            .connectivity
            .add_node(ConnectivityWeight::Band(BandWeight { from, to: None }));
        self.connectivity.update_edge(
            self.continent(from.into()).node_index(),
            band,
            ConnectivityLabel::Band,
        );
        BandIndex::new(band)
    }

    pub fn finish_band(&mut self, band: BandIndex, to: FixedDotIndex) {
        self.connectivity.update_edge(
            band.node_index(),
            self.continent(to.into()).node_index(),
            ConnectivityLabel::Band,
        );
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

    pub fn add_fixed_dot(&mut self, weight: FixedDotWeight) -> Result<FixedDotIndex, Infringement> {
        self.drawing.add_fixed_dot(weight)
    }

    pub fn add_zone_fixed_dot(
        &mut self,
        weight: FixedDotWeight,
        zone: GenericIndex<ZoneWeight>,
    ) -> Result<FixedDotIndex, Infringement> {
        let maybe_dot = self.drawing.add_fixed_dot(weight);

        if let Ok(dot) = maybe_dot {
            self.drawing.add_to_compound(dot, zone);
        }

        maybe_dot
    }

    pub fn add_fixed_seg(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
    ) -> Result<FixedSegIndex, Infringement> {
        self.drawing.add_fixed_seg(from, to, weight)
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
            self.drawing.add_to_compound(seg, zone);
        }

        maybe_seg
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

    pub fn band_from(&self, band: BandIndex) -> FixedDotIndex {
        todo!()
    }

    pub fn band_to(&self, band: BandIndex) -> Option<FixedDotIndex> {
        todo!()
    }

    pub fn band_length(&self, band: BandIndex) -> f64 {
        // TODO.
        0.0
    }

    pub fn continent(&self, dot: FixedDotIndex) -> ContinentIndex {
        // TODO.
        ContinentIndex::new(0.into())
    }

    pub fn zone_nodes(&self) -> impl Iterator<Item = GenericIndex<ZoneWeight>> + '_ {
        self.drawing.rtree().iter().filter_map(|wrapper| {
            if let NodeIndex::Compound(zone) = wrapper.data {
                Some(zone)
            } else {
                None
            }
        })
    }

    pub fn layer_zone_nodes(
        &self,
        layer: u64,
    ) -> impl Iterator<Item = GenericIndex<ZoneWeight>> + '_ {
        self.drawing
            .rtree()
            .locate_in_envelope_intersecting(&AABB::from_corners(
                [-f64::INFINITY, -f64::INFINITY, layer as f64],
                [f64::INFINITY, f64::INFINITY, layer as f64],
            ))
            .filter_map(|wrapper| {
                if let NodeIndex::Compound(zone) = wrapper.data {
                    Some(zone)
                } else {
                    None
                }
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

    pub fn zone_apex(&mut self, zone: GenericIndex<ZoneWeight>) -> FixedDotIndex {
        if let Some(apex) = self.zone(zone).maybe_apex() {
            apex
        } else {
            self.add_zone_fixed_dot(
                FixedDotWeight {
                    circle: Circle {
                        pos: self.zone(zone).shape().center(),
                        r: 100.0,
                    },
                    layer: self.zone(zone).layer(),
                    maybe_net: None,
                },
                zone,
            )
            .unwrap()
        }
    }

    pub fn drawing(&self) -> &Drawing<ZoneWeight, R> {
        &self.drawing
    }

    pub fn zone(&self, index: GenericIndex<ZoneWeight>) -> Zone<R> {
        Zone::new(index, self)
    }
}

impl<R: RulesTrait> CompoundManagerTrait<ZoneWeight, GenericIndex<ZoneWeight>> for Layout<R> {
    fn add_compound(&mut self, weight: ZoneWeight) -> GenericIndex<ZoneWeight> {
        self.drawing.add_compound(weight)
    }

    fn remove_compound(&mut self, compound: GenericIndex<ZoneWeight>) {
        self.drawing.remove_compound(compound);
    }

    fn add_to_compound<W>(
        &mut self,
        primitive: GenericIndex<W>,
        compound: GenericIndex<ZoneWeight>,
    ) {
        self.drawing.add_to_compound(primitive, compound);
    }

    fn compound_weight(&self, compound: GenericIndex<ZoneWeight>) -> ZoneWeight {
        self.drawing.compound_weight(compound)
    }

    fn compounds<W>(
        &self,
        node: GenericIndex<W>,
    ) -> impl Iterator<Item = GenericIndex<ZoneWeight>> {
        self.drawing.compounds(node)
    }
}
