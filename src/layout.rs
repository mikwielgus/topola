use contracts::{debug_ensures, debug_invariant};
use geo::Point;
use petgraph::stable_graph::StableDiGraph;
use petgraph::visit::EdgeRef;
use petgraph::Direction::Incoming;
use rstar::primitives::GeomWithData;
use rstar::{RTree, RTreeObject};
use spade::Triangulation;

use crate::band::Band;
use crate::bow::Bow;
use crate::graph::{
    BendIndex, BendWeight, DotIndex, DotWeight, Index, Interior, Label, Retag, SegIndex, SegWeight,
    Tag, TaggedIndex, Weight,
};
use crate::primitive::Primitive;
use crate::segbend::Segbend;
use crate::shape::{Shape, ShapeTrait};

pub type RTreeWrapper = GeomWithData<Shape, TaggedIndex>;

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
    pub fn remove_interior(&mut self, path: &impl Interior<TaggedIndex>) {
        for index in path.interior().iter().filter(|index| !index.is_dot()) {
            untag!(index, self.remove(*index));
        }

        // We must remove the dots only after the segs and bends because we need dots to calculate
        // the shapes, which we need to remove the segs and bends from the R-tree.

        for index in path.interior().iter().filter(|index| index.is_dot()) {
            untag!(index, self.remove(*index));
        }
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count() - 1))]
    pub fn remove<Weight: std::marker::Copy>(&mut self, index: Index<Weight>) {
        // Unnecessary retag. It should be possible to elide it.
        let weight = *self.graph.node_weight(index.index).unwrap();

        self.remove_from_rtree(weight.retag(index.index));
        self.graph.remove_node(index.index);
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count() + 1))]
    pub fn add_dot(&mut self, weight: DotWeight) -> Result<DotIndex, ()> {
        let dot = DotIndex::new(self.graph.add_node(Weight::Dot(weight)));

        self.insert_into_rtree(dot.tag());
        self.fail_and_remove_if_collides_except(dot, &[])?;

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

        self.graph.add_edge(from.index, seg.index, Label::End);
        self.graph.add_edge(seg.index, to.index, Label::End);

        self.insert_into_rtree(seg.tag());
        self.fail_and_remove_if_collides_except(seg, &[from.tag(), to.tag()])?;

        self.graph
            .node_weight_mut(from.index)
            .unwrap()
            .as_dot_mut()
            .unwrap()
            .net = weight.net;
        self.graph
            .node_weight_mut(to.index)
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
        around: TaggedIndex,
        weight: BendWeight,
    ) -> Result<BendIndex, ()> {
        match around {
            TaggedIndex::Dot(core) => self.add_core_bend(from, to, core, weight),
            TaggedIndex::Bend(around) => self.add_outer_bend(from, to, around, weight),
            TaggedIndex::Seg(..) => unreachable!(),
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

        self.graph.add_edge(from.index, bend.index, Label::End);
        self.graph.add_edge(bend.index, to.index, Label::End);
        self.graph.add_edge(bend.index, core.index, Label::Core);

        self.insert_into_rtree(bend.tag());
        self.fail_and_remove_if_collides_except(bend, &[from.tag(), to.tag(), core.tag()])?;
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
            .neighbors(inner.index)
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(inner.index, *ni).unwrap())
                    .unwrap()
                    .is_core()
            })
            .map(|ni| DotIndex::new(ni))
            .collect::<Vec<DotIndex>>()
            .first()
            .unwrap();

        let bend = BendIndex::new(self.graph.add_node(Weight::Bend(weight)));

        self.graph.add_edge(from.index, bend.index, Label::End);
        self.graph.add_edge(bend.index, to.index, Label::End);
        self.graph.add_edge(bend.index, core.index, Label::Core);
        self.graph.add_edge(inner.index, bend.index, Label::Outer);

        self.insert_into_rtree(bend.tag());
        self.fail_and_remove_if_collides_except(bend, &[from.tag(), to.tag(), core.tag()])?;
        Ok(bend)
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count())
        || self.graph.edge_count() == old(self.graph.edge_count() + 1))]
    pub fn reattach_bend(&mut self, bend: BendIndex, inner: BendIndex) {
        self.remove_from_rtree(bend.tag());

        if let Some(old_inner_edge) = self
            .graph
            .edges_directed(bend.index, Incoming)
            .filter(|edge| *edge.weight() == Label::Outer)
            .next()
        {
            self.graph.remove_edge(old_inner_edge.id());
        }

        self.graph.add_edge(inner.index, bend.index, Label::Outer);
        self.insert_into_rtree(bend.tag());
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn flip_bend(&mut self, bend: BendIndex) {
        self.remove_from_rtree(bend.tag());
        let cw = self
            .graph
            .node_weight(bend.index)
            .unwrap()
            .into_bend()
            .unwrap()
            .cw;
        self.graph
            .node_weight_mut(bend.index)
            .unwrap()
            .as_bend_mut()
            .unwrap()
            .cw = !cw;
        self.insert_into_rtree(bend.tag());
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
    fn fail_and_remove_if_collides_except<Weight: std::marker::Copy>(
        &mut self,
        index: Index<Weight>,
        except: &[TaggedIndex],
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

    fn nodes(&self) -> impl Iterator<Item = TaggedIndex> + '_ {
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
        self.remove_from_rtree(dot.tag());

        let mut dot_weight = self.primitive(dot).weight();
        let old_weight = dot_weight;

        dot_weight.circle.pos = to;
        *self.graph.node_weight_mut(dot.index).unwrap() = Weight::Dot(dot_weight);

        if let Some(..) = self.detect_collision_except(dot, &[]) {
            // Restore original state.
            *self.graph.node_weight_mut(dot.index).unwrap() = Weight::Dot(old_weight);

            self.insert_into_rtree(dot.tag());
            self.primitive(dot)
                .tagged_prev()
                .map(|prev| self.insert_into_rtree(prev));
            self.primitive(dot)
                .tagged_next()
                .map(|next| self.insert_into_rtree(next));
            return Err(());
        }

        self.insert_into_rtree(dot.tag());
        self.primitive(dot)
            .tagged_prev()
            .map(|prev| self.insert_into_rtree(prev));
        self.primitive(dot)
            .tagged_next()
            .map(|next| self.insert_into_rtree(next));

        Ok(())
    }

    pub fn primitive<Weight>(&self, index: Index<Weight>) -> Primitive<Weight> {
        Primitive::new(index, &self.graph)
    }

    fn detect_collision_except<Weight: std::marker::Copy>(
        &self,
        index: Index<Weight>,
        except: &[TaggedIndex],
    ) -> Option<TaggedIndex> {
        let primitive = self.primitive(index);
        let shape = primitive.shape();

        self.rtree
            .locate_in_envelope_intersecting(&RTreeObject::envelope(&shape))
            .filter(|wrapper| {
                let other_index = wrapper.data;
                !untag!(other_index, primitive.connectable(other_index))
            })
            .filter(|wrapper| !except.contains(&wrapper.data))
            .filter(|wrapper| shape.intersects(wrapper.geom()))
            .map(|wrapper| wrapper.data)
            .next()
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn insert_into_rtree(&mut self, index: TaggedIndex) {
        let shape = untag!(index, self.primitive(index).shape());
        self.rtree.insert(RTreeWrapper::new(shape, index));
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn remove_from_rtree(&mut self, index: TaggedIndex) {
        let shape = untag!(index, self.primitive(index).shape());
        let removed_element = self.rtree.remove(&RTreeWrapper::new(shape, index));
        debug_assert!(removed_element.is_some());
    }
}

impl Layout {
    fn test_envelopes(&self) -> bool {
        !self.rtree.iter().any(|wrapper| {
            let index = wrapper.data;
            let shape = untag!(index, Primitive::new(index, &self.graph).shape());
            let wrapper = RTreeWrapper::new(shape, index);
            !self
                .rtree
                .locate_in_envelope(&RTreeObject::envelope(&shape))
                .any(|w| *w == wrapper)
        })
    }
}
