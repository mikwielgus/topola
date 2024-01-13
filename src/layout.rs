use contracts::debug_invariant;
use enum_dispatch::enum_dispatch;
use geo::Point;
use petgraph::stable_graph::StableDiGraph;
use petgraph::visit::EdgeRef;
use petgraph::Direction::Incoming;
use rstar::primitives::GeomWithData;
use rstar::{RTree, RTreeObject};
use slab::Slab;
use thiserror::Error;

use crate::graph::{
    BendWeight, DotIndex, DotWeight, FixedBendIndex, FixedDotIndex, FixedDotWeight, FixedSegIndex,
    FixedSegWeight, GenericIndex, GetNet, GetNodeIndex, Index, Label, LooseBendIndex,
    LooseBendWeight, LooseDotIndex, LooseDotWeight, LooseSegIndex, LooseSegWeight, MakePrimitive,
    Retag, SegWeight, Weight, WraparoundableIndex,
};
use crate::guide::Guide;
use crate::math::NoTangents;
use crate::primitive::{
    GenericPrimitive, GetConnectable, GetCore, GetEnds, GetFirstRail, GetInnerOuter, GetInterior,
    GetOtherEnd, GetWeight, GetWraparound, MakeShape,
};
use crate::segbend::Segbend;
use crate::shape::{Shape, ShapeTrait};

pub type RTreeWrapper = GeomWithData<Shape, Index>;

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
pub struct Infringement(pub Shape, pub Index);

#[derive(Error, Debug, Clone, Copy)]
#[error("{0:?} collides with {1:?}")]
pub struct Collision(pub Shape, pub Index);

#[derive(Error, Debug, Clone, Copy)]
#[error("{1:?} is already connected to net {0}")]
pub struct AlreadyConnected(pub i64, pub Index);

#[derive(Debug, Clone, Copy)]
pub struct Band {
    pub net: i64,
    pub width: f64,
}

#[derive(Debug)]
pub struct Layout {
    rtree: RTree<RTreeWrapper>,
    pub bands: Slab<Band>,
    pub graph: StableDiGraph<Weight, Label, usize>,
}

#[debug_invariant(self.graph.node_count() == self.rtree.size())]
#[debug_invariant(self.test_envelopes())]
impl Layout {
    pub fn new() -> Self {
        Layout {
            rtree: RTree::new(),
            bands: Slab::new(),
            graph: StableDiGraph::default(),
        }
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count() - 4))]
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

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count() - 1))]
    fn remove(&mut self, index: Index) {
        // Unnecessary retag. It should be possible to elide it.
        let weight = *self.graph.node_weight(index.node_index()).unwrap();

        self.remove_from_rtree(weight.retag(index.node_index()));
        self.graph.remove_node(index.node_index());
    }

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn add_fixed_dot(&mut self, weight: FixedDotWeight) -> Result<FixedDotIndex, Infringement> {
        self.add_dot_infringably(weight, &[])
    }

    /*#[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn add_loose_dot(&mut self, weight: LooseDotWeight) -> Result<LooseDotIndex, ()> {
        self.add_dot_infringably(weight, &[])
    }*/

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    fn add_dot_infringably<W: DotWeight>(
        &mut self,
        weight: W,
        infringables: &[Index],
    ) -> Result<GenericIndex<W>, Infringement>
    where
        GenericIndex<W>: Into<Index> + Copy,
    {
        let dot = GenericIndex::<W>::new(self.graph.add_node(weight.into()));

        self.insert_into_rtree(dot.into());
        self.fail_and_remove_if_infringes_except(dot.into(), infringables)?;

        Ok(dot)
    }

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() == old(self.graph.edge_count() + 2))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_err() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn add_fixed_seg(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
    ) -> Result<FixedSegIndex, Infringement> {
        self.add_seg_infringably(from, to, weight, &[])
    }

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 4))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() >= old(self.graph.edge_count() + 5))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_err() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn insert_segbend(
        &mut self,
        from: DotIndex,
        around: WraparoundableIndex,
        dot_weight: LooseDotWeight,
        seg_weight: LooseSegWeight,
        bend_weight: LooseBendWeight,
    ) -> Result<Segbend, LayoutException> {
        let maybe_wraparound = match around {
            WraparoundableIndex::FixedDot(around) => self.primitive(around).wraparound(),
            WraparoundableIndex::FixedBend(around) => self.primitive(around).wraparound(),
            WraparoundableIndex::LooseBend(around) => self.primitive(around).wraparound(),
        };

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

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn inner_bow_and_outer_bow(&self, bend: LooseBendIndex) -> Vec<Index> {
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

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn inner_bow_and_outer_bows(&self, bend: LooseBendIndex) -> Vec<Index> {
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

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn this_and_wraparound_bow(&self, around: WraparoundableIndex) -> Vec<Index> {
        match around {
            WraparoundableIndex::FixedDot(dot) => {
                let mut v = vec![around.into()];
                if let Some(first_rail) = self.primitive(dot).first_rail() {
                    v.append(&mut self.bow(first_rail));
                }
                v
            }
            WraparoundableIndex::FixedBend(bend) => {
                let mut v = vec![around.into()];
                if let Some(first_rail) = self.primitive(bend).first_rail() {
                    v.append(&mut self.bow(first_rail));
                }
                v
            }
            WraparoundableIndex::LooseBend(bend) => {
                let mut v = self.bow(bend);
                if let Some(outer) = self.primitive(bend).outer() {
                    v.append(&mut self.bow(outer));
                }
                v
            }
        }
    }

    // XXX: Move this to primitives?
    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn bow(&self, bend: LooseBendIndex) -> Vec<Index> {
        let mut bow: Vec<Index> = vec![];
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

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn outer_bows(&self, bend: LooseBendIndex) -> Vec<Index> {
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

        /*let mut outer_bows = vec![];

        // XXX: Ugly all-same match.
        let mut maybe_rail = match around {
            WraparoundableIndex::FixedDot(around) => self.primitive(around).wraparound(),
            WraparoundableIndex::FixedBend(around) => self.primitive(around).wraparound(),
            WraparoundableIndex::LooseBend(around) => self.primitive(around).wraparound(),
        };

        while let Some(rail) = maybe_rail {
            let primitive = self.primitive(rail);
            outer_bows.push(rail.into());

            let ends = primitive.ends();
            outer_bows.push(ends.0.into());
            outer_bows.push(ends.1.into());

            outer_bows.push(self.primitive(ends.0).seg().unwrap().into());
            maybe_rail = primitive.outer();
        }

        outer_bows*/
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count())
        || self.graph.edge_count() == old(self.graph.edge_count() - 1)
        || self.graph.edge_count() == old(self.graph.edge_count() + 1))]
    fn reattach_bend(&mut self, bend: LooseBendIndex, maybe_new_inner: Option<LooseBendIndex>) {
        self.remove_from_rtree(bend.into());

        if let Some(old_inner_edge) = self
            .graph
            .edges_directed(bend.node_index(), Incoming)
            .filter(|edge| *edge.weight() == Label::Outer)
            .next()
        {
            self.graph.remove_edge(old_inner_edge.id());
        }

        if let Some(new_inner) = maybe_new_inner {
            self.graph
                .add_edge(new_inner.node_index(), bend.node_index(), Label::Outer);
        }

        self.insert_into_rtree(bend.into());
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
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

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 4))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() >= old(self.graph.edge_count() + 5))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_err() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn add_segbend(
        &mut self,
        from: DotIndex,
        around: WraparoundableIndex,
        dot_weight: LooseDotWeight,
        seg_weight: LooseSegWeight,
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

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 4))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() >= old(self.graph.edge_count() + 5))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_err() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    fn add_segbend_infringably(
        &mut self,
        from: DotIndex,
        around: WraparoundableIndex,
        dot_weight: LooseDotWeight,
        seg_weight: LooseSegWeight,
        bend_weight: LooseBendWeight,
        infringables: &[Index],
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

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() == old(self.graph.edge_count() + 2))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_err() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn add_loose_seg(
        &mut self,
        from: DotIndex,
        to: LooseDotIndex,
        weight: LooseSegWeight,
    ) -> Result<LooseSegIndex, Infringement> {
        self.add_seg_infringably(from, to, weight, &[])
    }

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() == old(self.graph.edge_count() + 2))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_err() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    fn add_seg_infringably<W: SegWeight>(
        &mut self,
        from: impl GetNodeIndex,
        to: impl GetNodeIndex,
        weight: W,
        infringables: &[Index],
    ) -> Result<GenericIndex<W>, Infringement>
    where
        GenericIndex<W>: Into<Index> + Copy,
    {
        let seg = GenericIndex::<W>::new(self.graph.add_node(weight.into()));

        self.graph
            .add_edge(from.node_index(), seg.node_index(), Label::Adjacent);
        self.graph
            .add_edge(seg.node_index(), to.node_index(), Label::Adjacent);

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
        around: Index,
        weight: FixedBendWeight,
    ) -> Result<FixedBendIndex, ()> {
        match around {
            Index::FixedDot(core) => self.add_core_bend(from, to, core, weight),
            Index::FixedBend(around) => self.add_outer_bend(from, to, around, weight),
            _ => unreachable!(),
        }
    }*/

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() == old(self.graph.edge_count() + 3)
        || self.graph.edge_count() == old(self.graph.edge_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_err() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    fn add_loose_bend_infringably(
        &mut self,
        from: LooseDotIndex,
        to: LooseDotIndex,
        around: WraparoundableIndex,
        weight: LooseBendWeight,
        infringables: &[Index],
    ) -> Result<LooseBendIndex, LayoutException> {
        // It makes no sense to wrap something around or under one of its connectables.
        let net = self.bands[weight.band].net;
        //
        if net == around.primitive(self).net() {
            return Err(AlreadyConnected(
                net,
                around.into(),
            ).into());
        }
        //
        if let Some(wraparound) = match around {
            WraparoundableIndex::FixedDot(around) => self.primitive(around).wraparound(),
            WraparoundableIndex::FixedBend(around) => self.primitive(around).wraparound(),
            WraparoundableIndex::LooseBend(around) => self.primitive(around).wraparound(),
        } {
            if net == wraparound.primitive(self).net() {
                return Err(AlreadyConnected(
                    net,
                    wraparound.into(),
                ).into());
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

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() == old(self.graph.edge_count() + 3))]
    #[debug_ensures(ret.is_err() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    fn add_core_bend_infringably<W: BendWeight>(
        &mut self,
        from: impl GetNodeIndex,
        to: impl GetNodeIndex,
        core: FixedDotIndex,
        weight: W,
        infringables: &[Index],
    ) -> Result<LooseBendIndex, Infringement>
    where
        GenericIndex<W>: Into<Index> + Copy,
    {
        let bend = LooseBendIndex::new(self.graph.add_node(weight.into()));

        self.graph
            .add_edge(from.node_index(), bend.node_index(), Label::Adjacent);
        self.graph
            .add_edge(bend.node_index(), to.node_index(), Label::Adjacent);
        self.graph
            .add_edge(bend.node_index(), core.node_index(), Label::Core);

        self.insert_into_rtree(bend.into());
        self.fail_and_remove_if_infringes_except(bend.into(), infringables)?;
        Ok(bend)
    }

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() == old(self.graph.edge_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    fn add_outer_bend_infringably<W: BendWeight>(
        &mut self,
        from: impl GetNodeIndex,
        to: impl GetNodeIndex,
        inner: impl GetNodeIndex,
        weight: W,
        infringables: &[Index],
    ) -> Result<LooseBendIndex, Infringement> {
        let core = *self
            .graph
            .neighbors(inner.node_index())
            .filter(|ni| {
                matches!(
                    self.graph
                        .edge_weight(self.graph.find_edge(inner.node_index(), *ni).unwrap())
                        .unwrap(),
                    Label::Core
                )
            })
            .map(|ni| FixedDotIndex::new(ni))
            .collect::<Vec<FixedDotIndex>>()
            .first()
            .unwrap();

        let bend = LooseBendIndex::new(self.graph.add_node(weight.into()));

        self.graph
            .add_edge(from.node_index(), bend.node_index(), Label::Adjacent);
        self.graph
            .add_edge(bend.node_index(), to.node_index(), Label::Adjacent);
        self.graph
            .add_edge(bend.node_index(), core.node_index(), Label::Core);
        self.graph
            .add_edge(inner.node_index(), bend.node_index(), Label::Outer);

        self.insert_into_rtree(bend.into());
        self.fail_and_remove_if_infringes_except(bend.into(), infringables)?;
        Ok(bend)
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn flip_bend(&mut self, bend: FixedBendIndex) {
        self.remove_from_rtree(bend.into());

        let Some(Weight::FixedBend(weight)) = self.graph.node_weight_mut(bend.node_index()) else {
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

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count() - 1))]
    fn fail_and_remove_if_infringes_except(
        &mut self,
        index: Index,
        except: &[Index],
    ) -> Result<(), Infringement> {
        if let Some(infringement) = self.detect_infringement_except(index, except) {
            self.remove(index);
            return Err(infringement);
        }
        Ok(())
    }

    pub fn nodes(&self) -> impl Iterator<Item = Index> + '_ {
        self.rtree.iter().map(|wrapper| wrapper.data)
    }

    pub fn shapes(&self) -> impl Iterator<Item = Shape> + '_ {
        self.nodes().map(|node| node.primitive(self).shape())
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    fn node_indices(&self) -> impl Iterator<Item = Index> + '_ {
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

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn move_dot_infringably(
        &mut self,
        dot: LooseDotIndex,
        to: Point,
        infringables: &[Index],
    ) -> Result<(), Infringement> {
        self.primitive(dot)
            .seg()
            .map(|seg| self.remove_from_rtree(seg.into()));
        self.remove_from_rtree(self.primitive(dot).bend().into());
        self.remove_from_rtree(dot.into());

        let mut dot_weight = self.primitive(dot).weight();
        let old_weight = dot_weight;

        dot_weight.circle.pos = to;
        *self.graph.node_weight_mut(dot.node_index()).unwrap() = Weight::LooseDot(dot_weight);

        if let Some(infringement) = self.detect_infringement_except(dot.into(), infringables) {
            // Restore original state.
            *self.graph.node_weight_mut(dot.node_index()).unwrap() = Weight::LooseDot(old_weight);

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

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn primitive<W>(&self, index: GenericIndex<W>) -> GenericPrimitive<W> {
        GenericPrimitive::new(index, self)
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn detect_infringement_except(&self, index: Index, except: &[Index]) -> Option<Infringement> {
        let shape = index.primitive(self).shape();

        self.rtree
            .locate_in_envelope_intersecting(&RTreeObject::envelope(&shape))
            .filter(|wrapper| {
                let other_index = wrapper.data;
                !index.primitive(self).connectable(other_index)
            })
            .filter(|wrapper| !except.contains(&wrapper.data))
            .filter(|wrapper| shape.intersects(wrapper.geom()))
            .map(|wrapper| wrapper.data)
            .next()
            .and_then(|infringee| Some(Infringement(shape, infringee)))
    }

    // TODO: Collision and infringement are the same for now. Change this.
    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn detect_collision(&self, index: Index) -> Option<Collision> {
        let shape = index.primitive(self).shape();

        self.rtree
            .locate_in_envelope_intersecting(&RTreeObject::envelope(&shape))
            .filter(|wrapper| {
                let other_index = wrapper.data;
                !index.primitive(self).connectable(other_index)
            })
            .filter(|wrapper| shape.intersects(wrapper.geom()))
            .map(|wrapper| wrapper.data)
            .next()
            .and_then(|collidee| Some(Collision(shape, collidee)))
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn insert_into_rtree(&mut self, index: Index) {
        let shape = index.primitive(self).shape();
        self.rtree.insert(RTreeWrapper::new(shape, index));
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn remove_from_rtree(&mut self, index: Index) {
        let shape = index.primitive(self).shape();
        let removed_element = self.rtree.remove(&RTreeWrapper::new(shape, index));
        debug_assert!(removed_element.is_some());
    }
}

impl Layout {
    fn test_envelopes(&self) -> bool {
        !self.rtree.iter().any(|wrapper| {
            let index = wrapper.data;
            let shape = index.primitive(self).shape();
            let wrapper = RTreeWrapper::new(shape, index);
            !self
                .rtree
                .locate_in_envelope(&RTreeObject::envelope(&shape))
                .any(|w| *w == wrapper)
        })
    }
}
