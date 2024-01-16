use contracts::{debug_ensures, debug_invariant};
use enum_dispatch::enum_dispatch;
use geo::Point;
use petgraph::stable_graph::StableDiGraph;
use petgraph::visit::EdgeRef;
use petgraph::Direction::Incoming;
use rstar::primitives::GeomWithData;
use rstar::{RTree, RTreeObject};
use thiserror::Error;

use crate::connectivity::{
    BandIndex, BandWeight, ComponentIndex, ComponentWeight, ConnectivityGraph, ConnectivityWeight,
    GetNet,
};
use crate::geometry::{
    BendWeight, DotIndex, DotWeight, FixedBendIndex, FixedDotIndex, FixedDotWeight, FixedSegIndex,
    FixedSegWeight, GeometryGraph, GeometryIndex, GeometryLabel, GeometryWeight, LoneLooseSegIndex,
    LoneLooseSegWeight, LooseBendIndex, LooseBendWeight, LooseDotIndex, LooseDotWeight,
    MakePrimitive, Retag, SegWeight, SeqLooseSegIndex, SeqLooseSegWeight,
};
use crate::graph::{GenericIndex, GetNodeIndex};
use crate::guide::Guide;
use crate::math::NoTangents;
use crate::primitive::{
    GenericPrimitive, GetConnectable, GetCore, GetEnds, GetInnerOuter, GetOtherEnd, GetWeight,
    MakeShape,
};
use crate::segbend::Segbend;
use crate::shape::{Shape, ShapeTrait};
use crate::wraparoundable::{GetWraparound, Wraparoundable, WraparoundableIndex};

pub type RTreeWrapper = GeomWithData<Shape, GeometryIndex>;

#[enum_dispatch]
#[derive(Error, Debug, Clone, Copy)]
pub enum LayoutException {
    #[error(transparent)]
    NoTangents(#[from] NoTangents),
    #[error(transparent)]
    Infringement(#[from] Infringement),
    #[error(transparent)]
    Collision(#[from] Collision),
    #[error(transparent)]
    AlreadyConnected(#[from] AlreadyConnected),
}

// TODO add real error messages + these should eventually use Display
#[derive(Error, Debug, Clone, Copy)]
#[error("{0:?} infringes on {1:?}")]
pub struct Infringement(pub Shape, pub GeometryIndex);

#[derive(Error, Debug, Clone, Copy)]
#[error("{0:?} collides with {1:?}")]
pub struct Collision(pub Shape, pub GeometryIndex);

#[derive(Error, Debug, Clone, Copy)]
#[error("{1:?} is already connected to net {0}")]
pub struct AlreadyConnected(pub i64, pub GeometryIndex);

#[derive(Debug)]
pub struct Layout {
    rtree: RTree<RTreeWrapper>,
    connectivity: ConnectivityGraph,
    geometry: GeometryGraph,
}

#[debug_invariant(self.geometry.node_count() == self.rtree.size())]
#[debug_invariant(self.test_envelopes())]
impl Layout {
    pub fn new() -> Self {
        Layout {
            rtree: RTree::new(),
            connectivity: StableDiGraph::default(),
            geometry: StableDiGraph::default(),
        }
    }

    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count() - 4))]
    pub fn remove_segbend(&mut self, segbend: &Segbend, face: LooseDotIndex) {
        let maybe_outer = self.primitive(segbend.bend).outer();

        // Removing a loose bend affects its outer bends.
        if let Some(outer) = maybe_outer {
            self.reattach_bend(outer, self.primitive(segbend.bend).inner());
        }

        self.remove(segbend.bend.into());
        self.remove(segbend.seg.into());

        // We must remove the dots only after the segs and bends because we need dots to calculate
        // the shapes, which we first need unchanged to remove the segs and bends from the R-tree.

        self.remove(face.into());
        self.remove(segbend.dot.into());

        if let Some(outer) = maybe_outer {
            self.update_this_and_outward_bows(outer).unwrap(); // Must never fail.
        }
    }

    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count() - 1))]
    fn remove(&mut self, node: GeometryIndex) {
        // Unnecessary retag. It should be possible to elide it.
        let weight = *self.geometry.node_weight(node.node_index()).unwrap();

        self.remove_from_rtree(weight.retag(node.node_index()));
        self.geometry.remove_node(node.node_index());
    }

    // TODO: This method shouldn't be public.
    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(self.geometry.edge_count() == old(self.geometry.edge_count()))]
    pub fn add_component(&mut self, net: i64) -> ComponentIndex {
        ComponentIndex::new(
            self.connectivity
                .add_node(ConnectivityWeight::Component(ComponentWeight { net })),
        )
    }

    // TODO: This method shouldn't be public.
    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(self.geometry.edge_count() == old(self.geometry.edge_count()))]
    pub fn add_band(&mut self, from: FixedDotIndex, width: f64) -> BandIndex {
        BandIndex::new(
            self.connectivity
                .add_node(ConnectivityWeight::Band(BandWeight {
                    width,
                    net: self.primitive(from).net(),
                })),
        )
    }

    #[debug_ensures(ret.is_ok() -> self.geometry.node_count() == old(self.geometry.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(self.geometry.edge_count() == old(self.geometry.edge_count()))]
    pub fn add_fixed_dot(&mut self, weight: FixedDotWeight) -> Result<FixedDotIndex, Infringement> {
        self.add_dot_infringably(weight, &[])
    }

    /*#[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn add_loose_dot(&mut self, weight: LooseDotWeight) -> Result<LooseDotIndex, ()> {
        self.add_dot_infringably(weight, &[])
    }*/

    #[debug_ensures(ret.is_ok() -> self.geometry.node_count() == old(self.geometry.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.geometry.node_count() == old(self.geometry.node_count()))]
    fn add_dot_infringably<W: DotWeight>(
        &mut self,
        weight: W,
        infringables: &[GeometryIndex],
    ) -> Result<GenericIndex<W>, Infringement>
    where
        GenericIndex<W>: Into<GeometryIndex> + Copy,
    {
        let dot = GenericIndex::<W>::new(self.geometry.add_node(weight.into()));

        self.insert_into_rtree(dot.into());
        self.fail_and_remove_if_infringes_except(dot.into(), infringables)?;

        Ok(dot)
    }

    #[debug_ensures(ret.is_ok() -> self.geometry.node_count() == old(self.geometry.node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.geometry.edge_count() == old(self.geometry.edge_count() + 2))]
    #[debug_ensures(ret.is_err() -> self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(ret.is_err() -> self.geometry.edge_count() == old(self.geometry.edge_count()))]
    pub fn add_fixed_seg(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
    ) -> Result<FixedSegIndex, Infringement> {
        self.add_seg_infringably(from, to, weight, &[])
    }

    #[debug_ensures(ret.is_ok() -> self.geometry.node_count() == old(self.geometry.node_count() + 4))]
    #[debug_ensures(ret.is_ok() -> self.geometry.edge_count() >= old(self.geometry.edge_count() + 5))]
    #[debug_ensures(ret.is_err() -> self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(ret.is_err() -> self.geometry.edge_count() == old(self.geometry.edge_count()))]
    pub fn insert_segbend(
        &mut self,
        from: DotIndex,
        around: WraparoundableIndex,
        dot_weight: LooseDotWeight,
        seg_weight: SeqLooseSegWeight,
        bend_weight: LooseBendWeight,
    ) -> Result<Segbend, LayoutException> {
        let maybe_wraparound = self.wraparoundable(around).wraparound();
        let mut infringables = self.this_and_wraparound_bow(around);

        if let Some(wraparound) = maybe_wraparound {
            infringables.append(&mut self.outer_bows(wraparound));
        }

        let segbend = self.add_segbend_infringably(
            from,
            around,
            dot_weight,
            seg_weight,
            bend_weight,
            &infringables,
        )?;

        if let Some(wraparound) = maybe_wraparound {
            self.reattach_bend(wraparound, Some(segbend.bend));
        }

        if let Some(outer) = self.primitive(segbend.bend).outer() {
            self.update_this_and_outward_bows(outer);
        }

        // Segs must not cross.
        if let Some(collision) = self.detect_collision(segbend.seg.into()) {
            let end = self.primitive(segbend.bend).other_end(segbend.dot);
            self.remove_segbend(&segbend, end.into());
            return Err(collision.into());
        }

        Ok::<Segbend, LayoutException>(segbend)
    }

    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(self.geometry.edge_count() == old(self.geometry.edge_count()))]
    fn inner_bow_and_outer_bow(&self, bend: LooseBendIndex) -> Vec<GeometryIndex> {
        let bend_primitive = self.primitive(bend);
        let mut v = vec![];

        if let Some(inner) = bend_primitive.inner() {
            v.append(&mut self.bow(inner.into()));
        } else {
            let core = bend_primitive.core();
            v.push(core.into());
        }

        if let Some(outer) = bend_primitive.outer() {
            v.append(&mut self.bow(outer.into()));
        }

        v
    }

    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(self.geometry.edge_count() == old(self.geometry.edge_count()))]
    fn inner_bow_and_outer_bows(&self, bend: LooseBendIndex) -> Vec<GeometryIndex> {
        let bend_primitive = self.primitive(bend);
        let mut v = vec![];

        if let Some(inner) = bend_primitive.inner() {
            v.append(&mut self.bow(inner.into()));
        } else {
            let core = bend_primitive.core();
            v.push(core.into());
        }

        let mut rail = bend;

        while let Some(outer) = self.primitive(rail).outer() {
            v.append(&mut self.bow(outer.into()));
            rail = outer;
        }

        v
    }

    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(self.geometry.edge_count() == old(self.geometry.edge_count()))]
    fn this_and_wraparound_bow(&self, around: WraparoundableIndex) -> Vec<GeometryIndex> {
        let mut v = match around {
            WraparoundableIndex::FixedDot(..) => vec![around.into()],
            WraparoundableIndex::FixedBend(..) => vec![around.into()],
            WraparoundableIndex::LooseBend(bend) => self.bow(bend),
        };
        if let Some(wraparound) = self.wraparoundable(around).wraparound() {
            v.append(&mut self.bow(wraparound));
        }
        v
    }

    // XXX: Move this to primitives?
    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(self.geometry.edge_count() == old(self.geometry.edge_count()))]
    fn bow(&self, bend: LooseBendIndex) -> Vec<GeometryIndex> {
        let mut bow: Vec<GeometryIndex> = vec![];
        bow.push(bend.into());

        let ends = self.primitive(bend).ends();
        bow.push(ends.0.into());
        bow.push(ends.1.into());

        if let Some(seg0) = self.primitive(ends.0).seg() {
            bow.push(seg0.into());
        }

        if let Some(seg1) = self.primitive(ends.1).seg() {
            bow.push(seg1.into());
        }

        bow
    }

    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(self.geometry.edge_count() == old(self.geometry.edge_count()))]
    fn outer_bows(&self, bend: LooseBendIndex) -> Vec<GeometryIndex> {
        let mut outer_bows = vec![];
        let mut rail = bend;

        while let Some(outer) = self.primitive(rail).outer() {
            let primitive = self.primitive(outer);

            outer_bows.push(outer.into());

            let ends = primitive.ends();
            outer_bows.push(ends.0.into());
            outer_bows.push(ends.1.into());

            outer_bows.push(self.primitive(ends.0).seg().unwrap().into());
            outer_bows.push(self.primitive(ends.1).seg().unwrap().into());

            rail = outer;
        }

        outer_bows
    }

    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(self.geometry.edge_count() == old(self.geometry.edge_count())
        || self.geometry.edge_count() == old(self.geometry.edge_count() - 1)
        || self.geometry.edge_count() == old(self.geometry.edge_count() + 1))]
    fn reattach_bend(&mut self, bend: LooseBendIndex, maybe_new_inner: Option<LooseBendIndex>) {
        self.remove_from_rtree(bend.into());

        if let Some(old_inner_edge) = self
            .geometry
            .edges_directed(bend.node_index(), Incoming)
            .filter(|edge| *edge.weight() == GeometryLabel::Outer)
            .next()
        {
            self.geometry.remove_edge(old_inner_edge.id());
        }

        if let Some(new_inner) = maybe_new_inner {
            self.geometry.add_edge(
                new_inner.node_index(),
                bend.node_index(),
                GeometryLabel::Outer,
            );
        }

        self.insert_into_rtree(bend.into());
    }

    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(self.geometry.edge_count() == old(self.geometry.edge_count()))]
    fn update_this_and_outward_bows(
        &mut self,
        around: LooseBendIndex,
    ) -> Result<(), LayoutException> {
        let mut maybe_rail = Some(around);

        while let Some(rail) = maybe_rail {
            let primitive = self.primitive(rail);
            let cw = primitive.weight().cw;
            let ends = primitive.ends();

            let rules = Default::default();
            let conditions = Default::default();
            let guide = Guide::new(self, &rules, &conditions);

            let from_head = guide.rear_head(ends.1);
            let to_head = guide.rear_head(ends.0);

            if let Some(inner) = primitive.inner() {
                let from = guide
                    .head_around_bend_segment(&from_head.into(), inner.into(), !cw, 6.0)?
                    .end_point();
                let to = guide
                    .head_around_bend_segment(&to_head.into(), inner.into(), cw, 6.0)?
                    .end_point();
                self.move_dot_infringably(ends.0, from, &self.inner_bow_and_outer_bows(rail))?;
                self.move_dot_infringably(ends.1, to, &self.inner_bow_and_outer_bows(rail))?;
            } else {
                let core = primitive.core();
                let from = guide
                    .head_around_dot_segment(&from_head.into(), core.into(), !cw, 6.0)?
                    .end_point();
                let to = guide
                    .head_around_dot_segment(&to_head.into(), core.into(), cw, 6.0)?
                    .end_point();
                self.move_dot_infringably(ends.0, from, &self.inner_bow_and_outer_bows(rail))?;
                self.move_dot_infringably(ends.1, to, &self.inner_bow_and_outer_bows(rail))?;
            }

            maybe_rail = self.primitive(rail).outer();
        }

        Ok::<(), LayoutException>(())
    }

    #[debug_ensures(ret.is_ok() -> self.geometry.node_count() == old(self.geometry.node_count() + 4))]
    #[debug_ensures(ret.is_ok() -> self.geometry.edge_count() >= old(self.geometry.edge_count() + 5))]
    #[debug_ensures(ret.is_err() -> self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(ret.is_err() -> self.geometry.edge_count() == old(self.geometry.edge_count()))]
    pub fn add_segbend(
        &mut self,
        from: DotIndex,
        around: WraparoundableIndex,
        dot_weight: LooseDotWeight,
        seg_weight: SeqLooseSegWeight,
        bend_weight: LooseBendWeight,
    ) -> Result<Segbend, LayoutException> {
        self.add_segbend_infringably(
            from,
            around,
            dot_weight,
            seg_weight,
            bend_weight,
            &self.this_and_wraparound_bow(around),
        )
    }

    #[debug_ensures(ret.is_ok() -> self.geometry.node_count() == old(self.geometry.node_count() + 4))]
    #[debug_ensures(ret.is_ok() -> self.geometry.edge_count() >= old(self.geometry.edge_count() + 5))]
    #[debug_ensures(ret.is_err() -> self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(ret.is_err() -> self.geometry.edge_count() == old(self.geometry.edge_count()))]
    fn add_segbend_infringably(
        &mut self,
        from: DotIndex,
        around: WraparoundableIndex,
        dot_weight: LooseDotWeight,
        seg_weight: SeqLooseSegWeight,
        bend_weight: LooseBendWeight,
        infringables: &[GeometryIndex],
    ) -> Result<Segbend, LayoutException> {
        let seg_to = self.add_dot_infringably(dot_weight, infringables)?;
        let seg = self
            .add_seg_infringably(from, seg_to, seg_weight, infringables)
            .map_err(|err| {
                self.remove(seg_to.into());
                err
            })?;

        let bend_to = self
            .add_dot_infringably(dot_weight, infringables)
            .map_err(|err| {
                self.remove(seg.into());
                self.remove(seg_to.into());
                err
            })?;
        let bend = self
            .add_loose_bend_infringably(seg_to, bend_to, around, bend_weight, infringables)
            .map_err(|err| {
                self.remove(bend_to.into());
                self.remove(seg.into());
                self.remove(seg_to.into());
                err
            })?;

        Ok::<Segbend, LayoutException>(Segbend {
            seg,
            dot: seg_to,
            bend,
        })
    }

    #[debug_ensures(ret.is_ok() -> self.geometry.node_count() == old(self.geometry.node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.geometry.edge_count() == old(self.geometry.edge_count() + 2))]
    #[debug_ensures(ret.is_err() -> self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(ret.is_err() -> self.geometry.edge_count() == old(self.geometry.edge_count()))]
    pub fn add_lone_loose_seg(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: LoneLooseSegWeight,
    ) -> Result<LoneLooseSegIndex, Infringement> {
        self.add_seg_infringably(from, to, weight, &[])
    }

    #[debug_ensures(ret.is_ok() -> self.geometry.node_count() == old(self.geometry.node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.geometry.edge_count() == old(self.geometry.edge_count() + 2))]
    #[debug_ensures(ret.is_err() -> self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(ret.is_err() -> self.geometry.edge_count() == old(self.geometry.edge_count()))]
    pub fn add_seq_loose_seg(
        &mut self,
        from: DotIndex,
        to: LooseDotIndex,
        weight: SeqLooseSegWeight,
    ) -> Result<SeqLooseSegIndex, Infringement> {
        self.add_seg_infringably(from, to, weight, &[])
    }

    #[debug_ensures(ret.is_ok() -> self.geometry.node_count() == old(self.geometry.node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.geometry.edge_count() == old(self.geometry.edge_count() + 2))]
    #[debug_ensures(ret.is_err() -> self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(ret.is_err() -> self.geometry.edge_count() == old(self.geometry.edge_count()))]
    fn add_seg_infringably<W: SegWeight>(
        &mut self,
        from: impl GetNodeIndex,
        to: impl GetNodeIndex,
        weight: W,
        infringables: &[GeometryIndex],
    ) -> Result<GenericIndex<W>, Infringement>
    where
        GenericIndex<W>: Into<GeometryIndex> + Copy,
    {
        let seg = GenericIndex::<W>::new(self.geometry.add_node(weight.into()));

        self.geometry
            .add_edge(from.node_index(), seg.node_index(), GeometryLabel::Adjacent);
        self.geometry
            .add_edge(seg.node_index(), to.node_index(), GeometryLabel::Adjacent);

        self.insert_into_rtree(seg.into());
        self.fail_and_remove_if_infringes_except(seg.into(), infringables)?;

        Ok(seg)
    }

    /*#[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    pub fn add_fixed_bend(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        around: GeometryIndex,
        weight: FixedBendWeight,
    ) -> Result<FixedBendIndex, ()> {
        match around {
            GeometryIndex::FixedDot(core) => self.add_core_bend(from, to, core, weight),
            GeometryIndex::FixedBend(around) => self.add_outer_bend(from, to, around, weight),
            _ => unreachable!(),
        }
    }*/

    #[debug_ensures(ret.is_ok() -> self.geometry.node_count() == old(self.geometry.node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.geometry.edge_count() == old(self.geometry.edge_count() + 3)
        || self.geometry.edge_count() == old(self.geometry.edge_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(ret.is_err() -> self.geometry.edge_count() == old(self.geometry.edge_count()))]
    fn add_loose_bend_infringably(
        &mut self,
        from: LooseDotIndex,
        to: LooseDotIndex,
        around: WraparoundableIndex,
        weight: LooseBendWeight,
        infringables: &[GeometryIndex],
    ) -> Result<LooseBendIndex, LayoutException> {
        // It makes no sense to wrap something around or under one of its connectables.
        let net = self
            .connectivity
            .node_weight(weight.band.node_index())
            .unwrap()
            .net();
        //
        if net == around.primitive(self).net() {
            return Err(AlreadyConnected(net, around.into()).into());
        }
        //
        if let Some(wraparound) = self.wraparoundable(around).wraparound() {
            if net == wraparound.primitive(self).net() {
                return Err(AlreadyConnected(net, wraparound.into()).into());
            }
        }

        match around {
            WraparoundableIndex::FixedDot(core) => self
                .add_core_bend_infringably(from, to, core, weight, infringables)
                .map_err(Into::into),
            WraparoundableIndex::FixedBend(around) => self
                .add_outer_bend_infringably(from, to, around, weight, infringables)
                .map_err(Into::into),
            WraparoundableIndex::LooseBend(around) => self
                .add_outer_bend_infringably(from, to, around, weight, infringables)
                .map_err(Into::into),
        }
    }

    #[debug_ensures(ret.is_ok() -> self.geometry.node_count() == old(self.geometry.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(ret.is_ok() -> self.geometry.edge_count() == old(self.geometry.edge_count() + 3))]
    #[debug_ensures(ret.is_err() -> self.geometry.edge_count() == old(self.geometry.edge_count()))]
    fn add_core_bend_infringably<W: BendWeight>(
        &mut self,
        from: impl GetNodeIndex,
        to: impl GetNodeIndex,
        core: FixedDotIndex,
        weight: W,
        infringables: &[GeometryIndex],
    ) -> Result<LooseBendIndex, Infringement>
    where
        GenericIndex<W>: Into<GeometryIndex> + Copy,
    {
        let bend = LooseBendIndex::new(self.geometry.add_node(weight.into()));

        self.geometry.add_edge(
            from.node_index(),
            bend.node_index(),
            GeometryLabel::Adjacent,
        );
        self.geometry
            .add_edge(bend.node_index(), to.node_index(), GeometryLabel::Adjacent);
        self.geometry
            .add_edge(bend.node_index(), core.node_index(), GeometryLabel::Core);

        self.insert_into_rtree(bend.into());
        self.fail_and_remove_if_infringes_except(bend.into(), infringables)?;
        Ok(bend)
    }

    #[debug_ensures(ret.is_ok() -> self.geometry.node_count() == old(self.geometry.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(ret.is_ok() -> self.geometry.edge_count() == old(self.geometry.edge_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.geometry.edge_count() == old(self.geometry.edge_count()))]
    fn add_outer_bend_infringably<W: BendWeight>(
        &mut self,
        from: impl GetNodeIndex,
        to: impl GetNodeIndex,
        inner: impl GetNodeIndex,
        weight: W,
        infringables: &[GeometryIndex],
    ) -> Result<LooseBendIndex, Infringement> {
        let core = *self
            .geometry
            .neighbors(inner.node_index())
            .filter(|ni| {
                matches!(
                    self.geometry
                        .edge_weight(self.geometry.find_edge(inner.node_index(), *ni).unwrap())
                        .unwrap(),
                    GeometryLabel::Core
                )
            })
            .map(|ni| FixedDotIndex::new(ni))
            .collect::<Vec<FixedDotIndex>>()
            .first()
            .unwrap();

        let bend = LooseBendIndex::new(self.geometry.add_node(weight.into()));

        self.geometry.add_edge(
            from.node_index(),
            bend.node_index(),
            GeometryLabel::Adjacent,
        );
        self.geometry
            .add_edge(bend.node_index(), to.node_index(), GeometryLabel::Adjacent);
        self.geometry
            .add_edge(bend.node_index(), core.node_index(), GeometryLabel::Core);
        self.geometry
            .add_edge(inner.node_index(), bend.node_index(), GeometryLabel::Outer);

        self.insert_into_rtree(bend.into());
        self.fail_and_remove_if_infringes_except(bend.into(), infringables)?;
        Ok(bend)
    }

    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(self.geometry.edge_count() == old(self.geometry.edge_count()))]
    pub fn flip_bend(&mut self, bend: FixedBendIndex) {
        self.remove_from_rtree(bend.into());

        let Some(GeometryWeight::FixedBend(weight)) =
            self.geometry.node_weight_mut(bend.node_index())
        else {
            unreachable!();
        };

        weight.cw = !weight.cw;

        self.insert_into_rtree(bend.into());
    }

    /*pub fn bow(&self, bend: LooseBendIndex) -> Bow {
        Bow::from_bend(bend, &self.graph)
    }*/

    pub fn segbend(&self, dot: LooseDotIndex) -> Segbend {
        Segbend::from_dot(dot, self)
    }

    #[debug_ensures(ret.is_ok() -> self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(ret.is_ok() -> self.geometry.edge_count() == old(self.geometry.edge_count()))]
    #[debug_ensures(ret.is_err() -> self.geometry.node_count() == old(self.geometry.node_count() - 1))]
    fn fail_and_remove_if_infringes_except(
        &mut self,
        node: GeometryIndex,
        except: &[GeometryIndex],
    ) -> Result<(), Infringement> {
        if let Some(infringement) = self.detect_infringement_except(node, except) {
            self.remove(node);
            return Err(infringement);
        }
        Ok(())
    }

    pub fn nodes(&self) -> impl Iterator<Item = GeometryIndex> + '_ {
        self.rtree.iter().map(|wrapper| wrapper.data)
    }

    pub fn shapes(&self) -> impl Iterator<Item = Shape> + '_ {
        self.nodes().map(|node| node.primitive(self).shape())
    }

    pub fn node_count(&self) -> usize {
        self.geometry.node_count()
    }

    fn node_indices(&self) -> impl Iterator<Item = GeometryIndex> + '_ {
        self.rtree.iter().map(|wrapper| wrapper.data)
    }
}

#[debug_invariant(self.test_envelopes())]
impl Layout {
    pub fn move_dot(&mut self, dot: LooseDotIndex, to: Point) -> Result<(), Infringement> {
        self.move_dot_infringably(
            dot,
            to,
            &self.inner_bow_and_outer_bow(self.primitive(dot).bend()),
        )
    }

    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(self.geometry.edge_count() == old(self.geometry.edge_count()))]
    pub fn move_dot_infringably(
        &mut self,
        dot: LooseDotIndex,
        to: Point,
        infringables: &[GeometryIndex],
    ) -> Result<(), Infringement> {
        self.primitive(dot)
            .seg()
            .map(|seg| self.remove_from_rtree(seg.into()));
        self.remove_from_rtree(self.primitive(dot).bend().into());
        self.remove_from_rtree(dot.into());

        let mut dot_weight = self.primitive(dot).weight();
        let old_weight = dot_weight;

        dot_weight.circle.pos = to;
        *self.geometry.node_weight_mut(dot.node_index()).unwrap() =
            GeometryWeight::LooseDot(dot_weight);

        if let Some(infringement) = self.detect_infringement_except(dot.into(), infringables) {
            // Restore original state.
            *self.geometry.node_weight_mut(dot.node_index()).unwrap() =
                GeometryWeight::LooseDot(old_weight);

            self.insert_into_rtree(dot.into());
            self.insert_into_rtree(self.primitive(dot).bend().into());
            self.primitive(dot)
                .seg()
                .map(|seg| self.insert_into_rtree(seg.into()));
            return Err(infringement);
        }

        self.insert_into_rtree(dot.into());
        self.insert_into_rtree(self.primitive(dot).bend().into());
        self.primitive(dot)
            .seg()
            .map(|seg| self.insert_into_rtree(seg.into()));

        Ok(())
    }

    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(self.geometry.edge_count() == old(self.geometry.edge_count()))]
    fn detect_infringement_except(
        &self,
        node: GeometryIndex,
        except: &[GeometryIndex],
    ) -> Option<Infringement> {
        let shape = node.primitive(self).shape();

        self.rtree
            .locate_in_envelope_intersecting(&RTreeObject::envelope(&shape))
            .filter(|wrapper| {
                let other_index = wrapper.data;
                !node.primitive(self).connectable(other_index)
            })
            .filter(|wrapper| !except.contains(&wrapper.data))
            .filter(|wrapper| shape.intersects(wrapper.geom()))
            .map(|wrapper| wrapper.data)
            .next()
            .and_then(|infringee| Some(Infringement(shape, infringee)))
    }

    // TODO: Collision and infringement are the same for now. Change this.
    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(self.geometry.edge_count() == old(self.geometry.edge_count()))]
    fn detect_collision(&self, node: GeometryIndex) -> Option<Collision> {
        let shape = node.primitive(self).shape();

        self.rtree
            .locate_in_envelope_intersecting(&RTreeObject::envelope(&shape))
            .filter(|wrapper| {
                let other_index = wrapper.data;
                !node.primitive(self).connectable(other_index)
            })
            .filter(|wrapper| shape.intersects(wrapper.geom()))
            .map(|wrapper| wrapper.data)
            .next()
            .and_then(|collidee| Some(Collision(shape, collidee)))
    }

    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(self.geometry.edge_count() == old(self.geometry.edge_count()))]
    fn insert_into_rtree(&mut self, node: GeometryIndex) {
        let shape = node.primitive(self).shape();
        self.rtree.insert(RTreeWrapper::new(shape, node));
    }

    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(self.geometry.edge_count() == old(self.geometry.edge_count()))]
    fn remove_from_rtree(&mut self, node: GeometryIndex) {
        let shape = node.primitive(self).shape();
        let removed_element = self.rtree.remove(&RTreeWrapper::new(shape, node));
        debug_assert!(removed_element.is_some());
    }
}

impl Layout {
    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(self.geometry.edge_count() == old(self.geometry.edge_count()))]
    pub fn connectivity(&self) -> &ConnectivityGraph {
        &self.connectivity
    }

    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(self.geometry.edge_count() == old(self.geometry.edge_count()))]
    pub fn geometry(&self) -> &GeometryGraph {
        &self.geometry
    }

    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(self.geometry.edge_count() == old(self.geometry.edge_count()))]
    pub fn primitive<W>(&self, node: GenericIndex<W>) -> GenericPrimitive<W> {
        GenericPrimitive::new(node, self)
    }

    #[debug_ensures(self.geometry.node_count() == old(self.geometry.node_count()))]
    #[debug_ensures(self.geometry.edge_count() == old(self.geometry.edge_count()))]
    pub fn wraparoundable(&self, node: WraparoundableIndex) -> Wraparoundable {
        Wraparoundable::new(node, self)
    }

    fn test_envelopes(&self) -> bool {
        !self.rtree.iter().any(|wrapper| {
            let node = wrapper.data;
            let shape = node.primitive(self).shape();
            let wrapper = RTreeWrapper::new(shape, node);
            !self
                .rtree
                .locate_in_envelope(&RTreeObject::envelope(&shape))
                .any(|w| *w == wrapper)
        })
    }
}
