use std::marker::PhantomData;

use contracts::debug_invariant;
use geo::Point;
use petgraph::stable_graph::StableDiGraph;
use rstar::{primitives::GeomWithData, RTree, RTreeObject, AABB};

use crate::{
    drawing::graph::{GetLayer, Retag},
    geometry::{
        shape::{Shape, ShapeTrait},
        BendWeightTrait, Compound, DotWeightTrait, Geometry, GeometryLabel, GetWidth,
        SegWeightTrait,
    },
    graph::{GenericIndex, GetNodeIndex},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bbox {
    aabb: AABB<[f64; 3]>,
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

type BboxedIndex<I> = GeomWithData<Bbox, I>;

#[derive(Debug)]
pub struct GeometryWithRtree<
    PW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<PI> + Copy,
    DW: DotWeightTrait<PW> + GetLayer,
    SW: SegWeightTrait<PW> + GetLayer,
    BW: BendWeightTrait<PW> + GetLayer,
    GW: Copy,
    PI: GetNodeIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Copy,
    DI: GetNodeIndex + Into<PI> + Copy,
    SI: GetNodeIndex + Into<PI> + Copy,
    BI: GetNodeIndex + Into<PI> + Copy,
> {
    geometry: Geometry<PW, DW, SW, BW, GW, PI, DI, SI, BI>,
    rtree: RTree<BboxedIndex<Compound<PI, GenericIndex<GW>>>>,
    layer_count: u64,
    weight_marker: PhantomData<PW>,
    dot_weight_marker: PhantomData<DW>,
    seg_weight_marker: PhantomData<SW>,
    bend_weight_marker: PhantomData<BW>,
    index_marker: PhantomData<PI>,
    dot_index_marker: PhantomData<DI>,
    seg_index_marker: PhantomData<SI>,
    bend_index_marker: PhantomData<BI>,
}

#[debug_invariant(self.test_envelopes())]
#[debug_invariant(self.geometry.graph().node_count() == self.rtree.size())]
impl<
        PW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<PI> + Copy,
        DW: DotWeightTrait<PW> + GetLayer,
        SW: SegWeightTrait<PW> + GetLayer,
        BW: BendWeightTrait<PW> + GetLayer,
        GW: Copy,
        PI: GetNodeIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + PartialEq + Copy,
        DI: GetNodeIndex + Into<PI> + Copy,
        SI: GetNodeIndex + Into<PI> + Copy,
        BI: GetNodeIndex + Into<PI> + Copy,
    > GeometryWithRtree<PW, DW, SW, BW, GW, PI, DI, SI, BI>
{
    pub fn new(layer_count: u64) -> Self {
        Self {
            geometry: Geometry::<PW, DW, SW, BW, GW, PI, DI, SI, BI>::new(),
            rtree: RTree::new(),
            layer_count,
            weight_marker: PhantomData,
            dot_weight_marker: PhantomData,
            seg_weight_marker: PhantomData,
            bend_weight_marker: PhantomData,
            index_marker: PhantomData,
            dot_index_marker: PhantomData,
            seg_index_marker: PhantomData,
            bend_index_marker: PhantomData,
        }
    }

    pub fn add_dot<W: DotWeightTrait<PW> + GetLayer>(&mut self, weight: W) -> GenericIndex<W>
    where
        GenericIndex<W>: Into<PI>,
    {
        let dot = self.geometry.add_dot(weight);
        self.rtree.insert(BboxedIndex::new(
            Bbox::new(
                self.geometry
                    .dot_shape(dot.into().try_into().unwrap_or_else(|_| unreachable!()))
                    .envelope_3d(0.0, weight.layer()),
            ),
            Compound::Primitive(dot.into()),
        ));
        dot
    }

    pub fn add_seg<W: SegWeightTrait<PW> + GetLayer>(
        &mut self,
        from: DI,
        to: DI,
        weight: W,
    ) -> GenericIndex<W>
    where
        GenericIndex<W>: Into<PI>,
    {
        let seg = self.geometry.add_seg(from, to, weight);
        self.rtree.insert(BboxedIndex::new(
            Bbox::new(
                self.geometry
                    .seg_shape(seg.into().try_into().unwrap_or_else(|_| unreachable!()))
                    .envelope_3d(0.0, weight.layer()),
            ),
            Compound::Primitive(seg.into()),
        ));
        seg
    }

    pub fn add_bend<W: BendWeightTrait<PW> + GetLayer>(
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
        self.rtree.insert(BboxedIndex::new(
            Bbox::new(
                self.geometry
                    .bend_shape(bend.into().try_into().unwrap_or_else(|_| unreachable!()))
                    .envelope_3d(0.0, weight.layer()),
            ),
            Compound::Primitive(bend.into()),
        ));
        bend
    }

    pub fn add_grouping(&mut self, weight: GW) -> GenericIndex<GW> {
        let grouping = self.geometry.add_grouping(weight);
        self.rtree.insert(BboxedIndex::new(
            Bbox::new(AABB::<[f64; 3]>::from_point([0.0, 0.0, -1.0].into())),
            Compound::Grouping(grouping),
        ));
        grouping
    }

    pub fn assign_to_grouping<W>(
        &mut self,
        primitive: GenericIndex<W>,
        grouping: GenericIndex<GW>,
    ) {
        self.geometry.assign_to_grouping(primitive, grouping)
    }

    pub fn remove_dot(&mut self, dot: DI) -> Result<(), ()> {
        if self.geometry.joined_segs(dot).next().is_some() {
            return Err(());
        }

        if self.geometry.joined_bends(dot).next().is_some() {
            return Err(());
        }

        self.rtree.remove(&self.make_dot_bbox(dot));
        self.geometry.remove_primitive(dot.into());
        Ok(())
    }

    pub fn remove_seg(&mut self, seg: SI) {
        self.rtree.remove(&self.make_seg_bbox(seg));
        self.geometry.remove_primitive(seg.into());
    }

    pub fn remove_bend(&mut self, bend: BI) {
        self.rtree.remove(&self.make_bend_bbox(bend));
        self.geometry.remove_primitive(bend.into());
    }

    pub fn remove_grouping(&mut self, grouping: GenericIndex<GW>) {
        self.rtree.remove(&self.make_grouping_bbox(grouping));
        self.geometry.remove_grouping(grouping);
    }

    pub fn move_dot(&mut self, dot: DI, to: Point) {
        for seg in self.geometry.joined_segs(dot) {
            self.rtree.remove(&self.make_seg_bbox(seg));
        }

        for bend in self.geometry.joined_bends(dot) {
            self.rtree.remove(&self.make_bend_bbox(bend));
        }

        self.rtree.remove(&self.make_dot_bbox(dot));
        self.geometry.move_dot(dot, to);
        self.rtree.insert(self.make_dot_bbox(dot));

        for bend in self.geometry.joined_bends(dot) {
            self.rtree.insert(self.make_bend_bbox(bend));
        }

        for seg in self.geometry.joined_segs(dot) {
            self.rtree.insert(self.make_seg_bbox(seg));
        }
    }

    pub fn shift_bend(&mut self, bend: BI, offset: f64) {
        let mut rail = bend;

        while let Some(outer) = self.geometry.outer(rail) {
            self.rtree.remove(&self.make_bend_bbox(outer));
            rail = outer;
        }

        self.rtree.remove(&self.make_bend_bbox(bend));
        self.geometry.shift_bend(bend, offset);
        self.rtree.insert(self.make_bend_bbox(bend));

        rail = bend;

        while let Some(outer) = self.geometry.outer(rail) {
            self.rtree.insert(self.make_bend_bbox(outer));
            rail = outer;
        }
    }

    pub fn flip_bend(&mut self, bend: BI) {
        // Does not affect the bbox because it covers the whole guidecircle.
        self.geometry.flip_bend(bend);
    }

    pub fn reattach_bend(&mut self, bend: BI, maybe_new_inner: Option<BI>) {
        let mut rail = bend;

        while let Some(outer) = self.geometry.outer(rail) {
            self.rtree.remove(&self.make_bend_bbox(outer));
            rail = outer;
        }

        self.rtree.remove(&self.make_bend_bbox(bend));
        self.geometry.reattach_bend(bend, maybe_new_inner);
        self.rtree.insert(self.make_bend_bbox(bend));

        rail = bend;

        while let Some(outer) = self.geometry.outer(rail) {
            self.rtree.insert(self.make_bend_bbox(outer));
            rail = outer;
        }
    }
}

impl<
        PW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<PI> + Copy,
        DW: DotWeightTrait<PW> + GetLayer,
        SW: SegWeightTrait<PW> + GetLayer,
        BW: BendWeightTrait<PW> + GetLayer,
        GW: Copy,
        PI: GetNodeIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + PartialEq + Copy,
        DI: GetNodeIndex + Into<PI> + Copy,
        SI: GetNodeIndex + Into<PI> + Copy,
        BI: GetNodeIndex + Into<PI> + Copy,
    > GeometryWithRtree<PW, DW, SW, BW, GW, PI, DI, SI, BI>
{
    fn make_dot_bbox(&self, dot: DI) -> BboxedIndex<Compound<PI, GenericIndex<GW>>> {
        BboxedIndex::new(
            Bbox::new(
                self.geometry
                    .dot_shape(dot)
                    .envelope_3d(0.0, self.layer(dot.into())),
            ),
            Compound::Primitive(dot.into()),
        )
    }

    fn make_seg_bbox(&self, seg: SI) -> BboxedIndex<Compound<PI, GenericIndex<GW>>> {
        BboxedIndex::new(
            Bbox::new(
                self.geometry
                    .seg_shape(seg)
                    .envelope_3d(0.0, self.layer(seg.into())),
            ),
            Compound::Primitive(seg.into()),
        )
    }

    fn make_bend_bbox(&self, bend: BI) -> BboxedIndex<Compound<PI, GenericIndex<GW>>> {
        BboxedIndex::new(
            Bbox::new(
                self.geometry
                    .bend_shape(bend)
                    .envelope_3d(0.0, self.layer(bend.into())),
            ),
            Compound::Primitive(bend.into()),
        )
    }

    fn make_grouping_bbox(
        &self,
        grouping: GenericIndex<GW>,
    ) -> BboxedIndex<Compound<PI, GenericIndex<GW>>> {
        BboxedIndex::new(
            Bbox::new(AABB::<[f64; 3]>::from_point([0.0, 0.0, -1.0].into())),
            Compound::Grouping(grouping),
        )
    }

    fn shape(&self, index: PI) -> Shape {
        if let Ok(dot) = <PI as TryInto<DI>>::try_into(index) {
            self.geometry.dot_shape(dot)
        } else if let Ok(seg) = <PI as TryInto<SI>>::try_into(index) {
            self.geometry.seg_shape(seg)
        } else if let Ok(bend) = <PI as TryInto<BI>>::try_into(index) {
            self.geometry.bend_shape(bend)
        } else {
            unreachable!();
        }
    }

    fn layer(&self, index: PI) -> u64 {
        if let Ok(dot) = <PI as TryInto<DI>>::try_into(index) {
            self.geometry.dot_weight(dot).layer()
        } else if let Ok(seg) = <PI as TryInto<SI>>::try_into(index) {
            self.geometry.seg_weight(seg).layer()
        } else if let Ok(bend) = <PI as TryInto<BI>>::try_into(index) {
            self.geometry.bend_weight(bend).layer()
        } else {
            unreachable!();
        }
    }

    pub fn geometry(&self) -> &Geometry<PW, DW, SW, BW, GW, PI, DI, SI, BI> {
        &self.geometry
    }

    pub fn rtree(&self) -> &RTree<BboxedIndex<Compound<PI, GenericIndex<GW>>>> {
        &self.rtree
    }

    pub fn graph(&self) -> &StableDiGraph<Compound<PW, GW>, GeometryLabel, usize> {
        self.geometry.graph()
    }

    fn test_envelopes(&self) -> bool {
        !self.rtree.iter().any(|wrapper| {
            // TODO: Test envelopes of groupings too.
            let Compound::Primitive(primitive_node) = wrapper.data else {
                return false;
            };
            let shape = self.shape(primitive_node);
            let layer = self.layer(primitive_node);
            let wrapper = BboxedIndex::new(
                Bbox::new(shape.envelope_3d(0.0, layer)),
                Compound::Primitive(primitive_node),
            );
            !self
                .rtree
                .locate_in_envelope(&shape.envelope_3d(0.0, layer))
                .any(|w| *w == wrapper)
        })
    }
}
