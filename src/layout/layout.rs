use geo::Point;
use petgraph::stable_graph::StableDiGraph;
use rstar::AABB;

use crate::{
    drawing::{
        bend::LooseBendWeight,
        dot::{DotIndex, FixedDotIndex, FixedDotWeight, LooseDotIndex, LooseDotWeight},
        graph::{PrimitiveIndex, Retag},
        rules::RulesTrait,
        seg::{
            FixedSegIndex, FixedSegWeight, LoneLooseSegIndex, LoneLooseSegWeight, SeqLooseSegIndex,
            SeqLooseSegWeight,
        },
        segbend::Segbend,
        Drawing, Infringement, LayoutException,
    },
    geometry::{
        grouping::GroupingManagerTrait, BendWeightTrait, DotWeightTrait, Geometry, GeometryLabel,
        GetWidth, Node, SegWeightTrait,
    },
    graph::{GenericIndex, GetNodeIndex},
    layout::{
        connectivity::{
            BandIndex, BandWeight, ConnectivityLabel, ConnectivityWeight, ContinentIndex,
        },
        zone::{PourZoneIndex, SolidZoneIndex, ZoneIndex, ZoneWeight},
    },
    wraparoundable::WraparoundableIndex,
};

pub struct Layout<R: RulesTrait> {
    drawing: Drawing<ZoneWeight, R>, // Shouldn't be public, but is for now because `Draw` needs it.
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
        zone: ZoneIndex,
    ) -> Result<FixedDotIndex, Infringement> {
        let maybe_dot = self.drawing.add_fixed_dot(weight);

        if let Ok(dot) = maybe_dot {
            self.drawing
                .assign_to_grouping(dot, GenericIndex::new(zone.node_index()));
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
        zone: ZoneIndex,
    ) -> Result<FixedSegIndex, Infringement> {
        let maybe_seg = self.add_fixed_seg(from, to, weight);

        if let Ok(seg) = maybe_seg {
            self.drawing
                .assign_to_grouping(seg, GenericIndex::new(zone.node_index()));
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

    pub fn zones(&self) -> impl Iterator<Item = ZoneIndex> + '_ {
        self.drawing.rtree().iter().filter_map(|wrapper| {
            if let Node::Grouping(zone) = wrapper.data {
                Some(match self.drawing.geometry().grouping_weight(zone) {
                    ZoneWeight::Solid(..) => {
                        ZoneIndex::Solid(SolidZoneIndex::new(zone.node_index()))
                    }
                    ZoneWeight::Pour(..) => ZoneIndex::Pour(PourZoneIndex::new(zone.node_index())),
                })
            } else {
                None
            }
        })
    }

    pub fn layer_zones(&self, layer: u64) -> impl Iterator<Item = ZoneIndex> + '_ {
        self.drawing
            .rtree()
            .locate_in_envelope_intersecting(&AABB::from_corners(
                [-f64::INFINITY, -f64::INFINITY, layer as f64],
                [f64::INFINITY, f64::INFINITY, layer as f64],
            ))
            .filter_map(|wrapper| {
                if let Node::Grouping(zone) = wrapper.data {
                    Some(match self.drawing.geometry().grouping_weight(zone) {
                        ZoneWeight::Solid(..) => {
                            ZoneIndex::Solid(SolidZoneIndex::new(zone.node_index()))
                        }
                        ZoneWeight::Pour(..) => {
                            ZoneIndex::Pour(PourZoneIndex::new(zone.node_index()))
                        }
                    })
                } else {
                    None
                }
            })
    }

    pub fn zone_members(&self, zone: ZoneIndex) -> impl Iterator<Item = PrimitiveIndex> + '_ {
        self.drawing
            .geometry()
            .grouping_members(GenericIndex::new(zone.node_index()))
    }

    pub fn drawing(&self) -> &Drawing<impl Copy, R> {
        &self.drawing
    }
}

impl<R: RulesTrait> GroupingManagerTrait<ZoneWeight, GenericIndex<ZoneWeight>> for Layout<R> {
    fn add_grouping(&mut self, weight: ZoneWeight) -> GenericIndex<ZoneWeight> {
        self.drawing.add_grouping(weight)
    }

    fn remove_grouping(&mut self, grouping: GenericIndex<ZoneWeight>) {
        self.drawing.remove_grouping(grouping);
    }

    fn assign_to_grouping<W>(
        &mut self,
        primitive: GenericIndex<W>,
        grouping: GenericIndex<ZoneWeight>,
    ) {
        self.drawing.assign_to_grouping(primitive, grouping);
    }

    fn groupings<W>(
        &self,
        node: GenericIndex<W>,
    ) -> impl Iterator<Item = GenericIndex<ZoneWeight>> {
        self.drawing.groupings(node)
    }
}
