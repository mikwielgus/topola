use std::marker::PhantomData;

use contracts::debug_invariant;
use geo::Point;
use petgraph::stable_graph::StableDiGraph;
use rstar::{primitives::GeomWithData, RTree, RTreeObject};

use crate::{
    graph::{GenericIndex, GetNodeIndex},
    layout::graph::{GetLayer, Retag},
};

use super::{
    shape::Shape, BendWeightTrait, DotWeightTrait, Geometry, GeometryLabel, GetWidth,
    SegWeightTrait,
};

type BboxedShapeAndIndex<GI> = GeomWithData<Shape, GI>;

#[derive(Debug)]
pub struct GeometryWithRtree<
    GW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<GI> + Copy,
    DW: DotWeightTrait<GW> + GetLayer + Copy,
    SW: SegWeightTrait<GW> + GetLayer + Copy,
    BW: BendWeightTrait<GW> + GetLayer + Copy,
    GI: GetNodeIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Copy,
    DI: GetNodeIndex + Into<GI> + Copy,
    SI: GetNodeIndex + Into<GI> + Copy,
    BI: GetNodeIndex + Into<GI> + Copy,
> {
    geometry: Geometry<GW, DW, SW, BW, GI, DI, SI, BI>,
    rtree: RTree<BboxedShapeAndIndex<GI>>,
    layer_count: u64,
    weight_marker: PhantomData<GW>,
    dot_weight_marker: PhantomData<DW>,
    seg_weight_marker: PhantomData<SW>,
    bend_weight_marker: PhantomData<BW>,
    index_marker: PhantomData<GI>,
    dot_index_marker: PhantomData<DI>,
    seg_index_marker: PhantomData<SI>,
    bend_index_marker: PhantomData<BI>,
}

#[debug_invariant(self.test_envelopes())]
#[debug_invariant(self.geometry.graph().node_count() == self.rtree.size())]
impl<
        GW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<GI> + Copy,
        DW: DotWeightTrait<GW> + GetLayer + Copy,
        SW: SegWeightTrait<GW> + GetLayer + Copy,
        BW: BendWeightTrait<GW> + GetLayer + Copy,
        GI: GetNodeIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + PartialEq + Copy,
        DI: GetNodeIndex + Into<GI> + Copy,
        SI: GetNodeIndex + Into<GI> + Copy,
        BI: GetNodeIndex + Into<GI> + Copy,
    > GeometryWithRtree<GW, DW, SW, BW, GI, DI, SI, BI>
{
    pub fn new(layer_count: u64) -> Self {
        Self {
            geometry: Geometry::<GW, DW, SW, BW, GI, DI, SI, BI>::new(),
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

    pub fn add_dot<W: DotWeightTrait<GW>>(&mut self, weight: W) -> GenericIndex<W>
    where
        GenericIndex<W>: Into<GI>,
    {
        let dot = self.geometry.add_dot(weight);
        self.rtree.insert(BboxedShapeAndIndex::new(
            self.geometry
                .dot_shape(dot.into().try_into().unwrap_or_else(|_| unreachable!())),
            dot.into(),
        ));
        dot
    }

    pub fn add_seg<W: SegWeightTrait<GW>>(&mut self, from: DI, to: DI, weight: W) -> GenericIndex<W>
    where
        GenericIndex<W>: Into<GI>,
    {
        let seg = self.geometry.add_seg(from, to, weight);
        self.rtree.insert(BboxedShapeAndIndex::new(
            self.geometry
                .seg_shape(seg.into().try_into().unwrap_or_else(|_| unreachable!())),
            seg.into(),
        ));
        seg
    }

    pub fn add_bend<W: BendWeightTrait<GW>>(
        &mut self,
        from: DI,
        to: DI,
        core: DI,
        weight: W,
    ) -> GenericIndex<W>
    where
        GenericIndex<W>: Into<GI>,
    {
        let bend = self.geometry.add_bend(from, to, core, weight);
        self.rtree.insert(BboxedShapeAndIndex::new(
            self.geometry
                .bend_shape(bend.into().try_into().unwrap_or_else(|_| unreachable!())),
            bend.into(),
        ));
        bend
    }

    pub fn remove_dot(&mut self, dot: DI) -> Result<(), ()> {
        if self.geometry.joined_segs(dot).next().is_some() {
            return Err(());
        }

        if self.geometry.joined_bends(dot).next().is_some() {
            return Err(());
        }

        self.rtree.remove(&self.make_dot_bbox(dot));
        self.geometry.remove(dot.into());
        Ok(())
    }

    pub fn remove_seg(&mut self, seg: SI) {
        self.rtree.remove(&self.make_seg_bbox(seg));
        self.geometry.remove(seg.into());
    }

    pub fn remove_bend(&mut self, bend: BI) {
        self.rtree.remove(&self.make_bend_bbox(bend));
        self.geometry.remove(bend.into());
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
        GW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<GI> + Copy,
        DW: DotWeightTrait<GW> + GetLayer + Copy,
        SW: SegWeightTrait<GW> + GetLayer + Copy,
        BW: BendWeightTrait<GW> + GetLayer + Copy,
        GI: GetNodeIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + PartialEq + Copy,
        DI: GetNodeIndex + Into<GI> + Copy,
        SI: GetNodeIndex + Into<GI> + Copy,
        BI: GetNodeIndex + Into<GI> + Copy,
    > GeometryWithRtree<GW, DW, SW, BW, GI, DI, SI, BI>
{
    fn make_dot_bbox(&self, dot: DI) -> BboxedShapeAndIndex<GI> {
        BboxedShapeAndIndex::new(self.geometry.dot_shape(dot), dot.into())
    }

    fn make_seg_bbox(&self, seg: SI) -> BboxedShapeAndIndex<GI> {
        BboxedShapeAndIndex::new(self.geometry.seg_shape(seg), seg.into())
    }

    fn make_bend_bbox(&self, bend: BI) -> BboxedShapeAndIndex<GI> {
        BboxedShapeAndIndex::new(self.geometry.bend_shape(bend), bend.into())
    }

    fn shape(&self, index: GI) -> Shape {
        if let Ok(dot) = <GI as TryInto<DI>>::try_into(index) {
            self.geometry.dot_shape(dot)
        } else if let Ok(seg) = <GI as TryInto<SI>>::try_into(index) {
            self.geometry.seg_shape(seg)
        } else if let Ok(bend) = <GI as TryInto<BI>>::try_into(index) {
            self.geometry.bend_shape(bend)
        } else {
            unreachable!();
        }
    }

    pub fn geometry(&self) -> &Geometry<GW, DW, SW, BW, GI, DI, SI, BI> {
        &self.geometry
    }

    pub fn rtree(&self) -> &RTree<BboxedShapeAndIndex<GI>> {
        &self.rtree
    }

    pub fn graph(&self) -> &StableDiGraph<GW, GeometryLabel, usize> {
        self.geometry.graph()
    }

    fn test_envelopes(&self) -> bool {
        !self.rtree.iter().any(|wrapper| {
            let node = wrapper.data;
            let shape = self.shape(node);
            let wrapper = BboxedShapeAndIndex::new(shape, node);
            !self
                .rtree
                .locate_in_envelope(&RTreeObject::envelope(&shape))
                .any(|w| *w == wrapper)
        })
    }
}
