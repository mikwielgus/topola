use contracts::debug_invariant;
use geo::Point;
use petgraph::stable_graph::StableDiGraph;
use petgraph::visit::EdgeRef;
use petgraph::Direction::Incoming;
use rstar::primitives::GeomWithData;
use rstar::{RTree, RTreeObject};

use crate::band::Band;
use crate::bow::Bow;
use crate::graph::{
    BendIndex, BendWeight, DotIndex, DotWeight, GenericIndex, GetNodeIndex, Index, Interior, Label,
    Retag, SegIndex, SegWeight, Weight,
};
use crate::primitive::{GenericPrimitive, MakeShape};
use crate::segbend::Segbend;
use crate::shape::{Shape, ShapeTrait};

pub type RTreeWrapper = GeomWithData<Shape, Index>;

pub struct Layout {
    rtree: RTree<RTreeWrapper>,
    pub graph: StableDiGraph<Weight, Label, usize>,
}

#[debug_invariant(self.graph.node_count() == self.rtree.size())]
#[debug_invariant(self.test_envelopes())]
impl Layout {
    pub fn new() -> Self {
        Layout {
            rtree: RTree::new(),
            graph: StableDiGraph::default(),
        }
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count() - path.interior().len()))]
    pub fn remove_interior(&mut self, path: &impl Interior<Index>) {
        for index in path.interior().into_iter().filter(|index| !index.is_dot()) {
            self.remove(index);
        }

        // We must remove the dots only after the segs and bends because we need dots to calculate
        // the shapes, which we need to remove the segs and bends from the R-tree.

        for index in path.interior().into_iter().filter(|index| index.is_dot()) {
            self.remove(index);
        }
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count() - 1))]
    pub fn remove(&mut self, index: Index) {
        // Unnecessary retag. It should be possible to elide it.
        let weight = *self.graph.node_weight(index.node_index()).unwrap();

        self.remove_from_rtree(weight.retag(index.node_index()));
        self.graph.remove_node(index.node_index());
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count() + 1))]
    pub fn add_dot(&mut self, weight: DotWeight) -> Result<DotIndex, ()> {
        let dot = DotIndex::new(self.graph.add_node(Weight::Dot(weight)));

        self.insert_into_rtree(Index::Dot(dot));
        self.fail_and_remove_if_collides_except(Index::Dot(dot), &[])?;

        Ok(dot)
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count() + 2))]
    pub fn add_seg(
        &mut self,
        from: DotIndex,
        to: DotIndex,
        weight: SegWeight,
    ) -> Result<SegIndex, ()> {
        let seg = SegIndex::new(self.graph.add_node(Weight::Seg(weight)));

        self.graph
            .add_edge(from.node_index(), seg.node_index(), Label::End);
        self.graph
            .add_edge(seg.node_index(), to.node_index(), Label::End);

        self.insert_into_rtree(Index::Seg(seg));
        self.fail_and_remove_if_collides_except(Index::Seg(seg), &[])?;

        self.graph
            .node_weight_mut(from.node_index())
            .unwrap()
            .as_dot_mut()
            .unwrap()
            .net = weight.net;
        self.graph
            .node_weight_mut(to.node_index())
            .unwrap()
            .as_dot_mut()
            .unwrap()
            .net = weight.net;

        Ok(seg)
    }

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    pub fn add_bend(
        &mut self,
        from: DotIndex,
        to: DotIndex,
        around: Index,
        weight: BendWeight,
    ) -> Result<BendIndex, ()> {
        match around {
            Index::Dot(core) => self.add_core_bend(from, to, core, weight),
            Index::Bend(around) => self.add_outer_bend(from, to, around, weight),
            Index::Seg(..) => unreachable!(),
        }
    }

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() == old(self.graph.edge_count() + 3))]
    #[debug_ensures(ret.is_err() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn add_core_bend(
        &mut self,
        from: DotIndex,
        to: DotIndex,
        core: DotIndex,
        weight: BendWeight,
    ) -> Result<BendIndex, ()> {
        let bend = BendIndex::new(self.graph.add_node(Weight::Bend(weight)));

        self.graph
            .add_edge(from.node_index(), bend.node_index(), Label::End);
        self.graph
            .add_edge(bend.node_index(), to.node_index(), Label::End);
        self.graph
            .add_edge(bend.node_index(), core.node_index(), Label::Core);

        self.insert_into_rtree(Index::Bend(bend));
        self.fail_and_remove_if_collides_except(Index::Bend(bend), &[Index::Dot(core)])?;
        Ok(bend)
    }

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() == old(self.graph.edge_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn add_outer_bend(
        &mut self,
        from: DotIndex,
        to: DotIndex,
        inner: BendIndex,
        weight: BendWeight,
    ) -> Result<BendIndex, ()> {
        let core = *self
            .graph
            .neighbors(inner.node_index())
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(inner.node_index(), *ni).unwrap())
                    .unwrap()
                    .is_core()
            })
            .map(|ni| DotIndex::new(ni))
            .collect::<Vec<DotIndex>>()
            .first()
            .unwrap();

        let bend = BendIndex::new(self.graph.add_node(Weight::Bend(weight)));

        self.graph
            .add_edge(from.node_index(), bend.node_index(), Label::End);
        self.graph
            .add_edge(bend.node_index(), to.node_index(), Label::End);
        self.graph
            .add_edge(bend.node_index(), core.node_index(), Label::Core);
        self.graph
            .add_edge(inner.node_index(), bend.node_index(), Label::Outer);

        self.insert_into_rtree(Index::Bend(bend));
        self.fail_and_remove_if_collides_except(Index::Bend(bend), &[Index::Dot(core)])?;
        Ok(bend)
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count())
        || self.graph.edge_count() == old(self.graph.edge_count() + 1))]
    pub fn reattach_bend(&mut self, bend: BendIndex, inner: BendIndex) {
        self.remove_from_rtree(Index::Bend(bend));

        if let Some(old_inner_edge) = self
            .graph
            .edges_directed(bend.node_index(), Incoming)
            .filter(|edge| *edge.weight() == Label::Outer)
            .next()
        {
            self.graph.remove_edge(old_inner_edge.id());
        }

        self.graph
            .add_edge(inner.node_index(), bend.node_index(), Label::Outer);
        self.insert_into_rtree(Index::Bend(bend));
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn flip_bend(&mut self, bend: BendIndex) {
        self.remove_from_rtree(Index::Bend(bend));
        let cw = self
            .graph
            .node_weight(bend.node_index())
            .unwrap()
            .into_bend()
            .unwrap()
            .cw;
        self.graph
            .node_weight_mut(bend.node_index())
            .unwrap()
            .as_bend_mut()
            .unwrap()
            .cw = !cw;
        self.insert_into_rtree(Index::Bend(bend));
    }

    pub fn bow(&self, bend: BendIndex) -> Bow {
        Bow::from_bend(bend, &self.graph)
    }

    pub fn prev_segbend(&self, dot: DotIndex) -> Option<Segbend> {
        Segbend::from_dot_prev(dot, &self.graph)
    }

    pub fn next_segbend(&self, dot: DotIndex) -> Option<Segbend> {
        Segbend::from_dot_next(dot, &self.graph)
    }

    pub fn prev_band(&self, to: DotIndex) -> Option<Band> {
        Band::from_dot_prev(to, &self.graph)
    }

    pub fn next_band(&self, from: DotIndex) -> Option<Band> {
        Band::from_dot_next(from, &self.graph)
    }

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count() - 1))]
    fn fail_and_remove_if_collides_except(
        &mut self,
        index: Index,
        except: &[Index],
    ) -> Result<(), ()> {
        if let Some(..) = self.detect_collision_except(index, except) {
            self.remove(index);
            return Err(());
        }
        Ok(())
    }

    pub fn dots(&self) -> impl Iterator<Item = DotIndex> + '_ {
        self.nodes().filter_map(|ni| ni.as_dot().map(|di| *di))
    }

    pub fn shapes(&self) -> impl Iterator<Item = Shape> + '_ {
        self.nodes()
            .map(|ni| untag!(ni, self.primitive(ni).shape()))
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    fn nodes(&self) -> impl Iterator<Item = Index> + '_ {
        self.rtree.iter().map(|wrapper| wrapper.data)
    }
}

#[debug_invariant(self.test_envelopes())]
impl Layout {
    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn move_dot(&mut self, dot: DotIndex, to: Point) -> Result<(), ()> {
        self.primitive(dot)
            .tagged_prev()
            .map(|prev| self.remove_from_rtree(prev));
        self.primitive(dot)
            .tagged_next()
            .map(|next| self.remove_from_rtree(next));
        self.remove_from_rtree(Index::Dot(dot));

        let mut dot_weight = self.primitive(dot).weight();
        let old_weight = dot_weight;

        dot_weight.circle.pos = to;
        *self.graph.node_weight_mut(dot.node_index()).unwrap() = Weight::Dot(dot_weight);

        if let Some(..) = self.detect_collision_except(Index::Dot(dot), &[]) {
            // Restore original state.
            *self.graph.node_weight_mut(dot.node_index()).unwrap() = Weight::Dot(old_weight);

            self.insert_into_rtree(Index::Dot(dot));
            self.primitive(dot)
                .tagged_prev()
                .map(|prev| self.insert_into_rtree(prev));
            self.primitive(dot)
                .tagged_next()
                .map(|next| self.insert_into_rtree(next));
            return Err(());
        }

        self.insert_into_rtree(Index::Dot(dot));
        self.primitive(dot)
            .tagged_prev()
            .map(|prev| self.insert_into_rtree(prev));
        self.primitive(dot)
            .tagged_next()
            .map(|next| self.insert_into_rtree(next));

        Ok(())
    }

    pub fn primitive<W>(&self, index: GenericIndex<W>) -> GenericPrimitive<W> {
        GenericPrimitive::new(index, &self.graph)
    }

    fn detect_collision_except(&self, index: Index, except: &[Index]) -> Option<Index> {
        let shape = untag!(index, self.primitive(index).shape());

        self.rtree
            .locate_in_envelope_intersecting(&RTreeObject::envelope(&shape))
            .filter(|wrapper| {
                let other_index = wrapper.data;
                !untag!(
                    other_index,
                    untag!(index, self.primitive(index).connectable(other_index))
                )
            })
            .filter(|wrapper| !except.contains(&wrapper.data))
            .filter(|wrapper| shape.intersects(wrapper.geom()))
            .map(|wrapper| wrapper.data)
            .next()
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn insert_into_rtree(&mut self, index: Index) {
        let shape = untag!(index, self.primitive(index).shape());
        self.rtree.insert(RTreeWrapper::new(shape, index));
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn remove_from_rtree(&mut self, index: Index) {
        let shape = untag!(index, self.primitive(index).shape());
        let removed_element = self.rtree.remove(&RTreeWrapper::new(shape, index));
        debug_assert!(removed_element.is_some());
    }
}

impl Layout {
    fn test_envelopes(&self) -> bool {
        !self.rtree.iter().any(|wrapper| {
            let index = wrapper.data;
            let shape = untag!(index, GenericPrimitive::new(index, &self.graph).shape());
            let wrapper = RTreeWrapper::new(shape, index);
            !self
                .rtree
                .locate_in_envelope(&RTreeObject::envelope(&shape))
                .any(|w| *w == wrapper)
        })
    }
}
