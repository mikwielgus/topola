use geo::Point;
use petgraph::stable_graph::StableDiGraph;
use rstar::RTree;
use rstar::primitives::GeomWithData;

use crate::primitive::Primitive;
use crate::shape::Shape;
use crate::graph::{Tag, TaggedIndex, DotIndex, SegIndex, BendIndex, Index, TaggedWeight, DotWeight, SegWeight, BendWeight, Label};

pub type RTreeWrapper = GeomWithData<Shape, TaggedIndex>;

pub struct Mesh {
    pub rtree: RTree<RTreeWrapper>,
    pub graph: StableDiGraph<TaggedWeight, Label, usize>,
}

impl Mesh {
    pub fn new() -> Self {
        Mesh {
            rtree: RTree::new(),
            graph: StableDiGraph::default(),
        }
    }

    pub fn add_dot(&mut self, weight: DotWeight) -> DotIndex {
        let dot = DotIndex::new(self.graph.add_node(TaggedWeight::Dot(weight)));
        self.rtree.insert(RTreeWrapper::new(self.primitive(dot).shape(), TaggedIndex::Dot(dot)));
        dot
    }

    pub fn remove_dot(&mut self, dot: DotIndex) {
        self.rtree.remove(&RTreeWrapper::new(self.primitive(dot).shape(), TaggedIndex::Dot(dot)));
        self.graph.remove_node(dot.index);
    }

    pub fn add_seg(&mut self, from: DotIndex, to: DotIndex, weight: SegWeight) -> SegIndex {
        let seg = SegIndex::new(self.graph.add_node(TaggedWeight::Seg(weight)));
        self.graph.add_edge(seg.index, from.index, Label::End);
        self.graph.add_edge(seg.index, to.index, Label::End);

        self.rtree.insert(RTreeWrapper::new(self.primitive(seg).shape(), TaggedIndex::Seg(seg)));
        seg
    }

    pub fn remove_seg(&mut self, seg: SegIndex) {
        self.rtree.remove(&RTreeWrapper::new(self.primitive(seg).shape(), TaggedIndex::Seg(seg)));
        self.graph.remove_node(seg.index);
    }

    pub fn add_bend(&mut self, from: DotIndex, to: DotIndex, around: TaggedIndex, weight: BendWeight) -> BendIndex {
        match around {
            TaggedIndex::Dot(core) =>
                self.add_core_bend(from, to, core, weight),
            TaggedIndex::Bend(around) =>
                self.add_outer_bend(from, to, around, weight),
            TaggedIndex::Seg(..) => unreachable!(),
        }
    }

    pub fn add_core_bend(&mut self, from: DotIndex, to: DotIndex, core: DotIndex, weight: BendWeight) -> BendIndex {
        let bend = BendIndex::new(self.graph.add_node(TaggedWeight::Bend(weight)));
        self.graph.add_edge(bend.index, from.index, Label::End);
        self.graph.add_edge(bend.index, to.index, Label::End);
        self.graph.add_edge(bend.index, core.index, Label::Core);

        self.rtree.insert(RTreeWrapper::new(self.primitive(bend).shape(), TaggedIndex::Bend(bend)));
        bend
    }

    pub fn add_outer_bend(&mut self, from: DotIndex, to: DotIndex, inner: BendIndex, weight: BendWeight) -> BendIndex {
        let core = *self.graph.neighbors(inner.index)
            .filter(|ni| self.graph.edge_weight(self.graph.find_edge(inner.index, *ni).unwrap()).unwrap().is_core())
            .map(|ni| DotIndex::new(ni))
            .collect::<Vec<DotIndex>>()
            .first()
            .unwrap();
        let bend = self.add_core_bend(from, to, core, weight);
        self.graph.add_edge(inner.index, bend.index, Label::Outer);
        bend
    }

    pub fn remove_bend(&mut self, bend: BendIndex) {
        self.rtree.remove(&RTreeWrapper::new(self.primitive(bend).shape(), TaggedIndex::Bend(bend)));
        self.graph.remove_node(bend.index);
    }

    pub fn extend_bend(&mut self, bend: BendIndex, dot: DotIndex, to: Point) {
        self.remove_from_rtree(bend.tag());
        self.remove_from_rtree(dot.tag());
        
        let mut dot_weight = self.primitive(dot).weight();
        dot_weight.circle.pos = to;
        *self.graph.node_weight_mut(dot.index).unwrap() = TaggedWeight::Dot(dot_weight);

        self.insert_into_rtree(dot.tag());
        self.insert_into_rtree(bend.tag());
    }

    /*pub fn shift_bend(&mut self, bend: BendIndex, offset: f64) {
        
    }*/

    /*pub fn position_bend(&mut self, bend: BendIndex, uI*/

    //pub fn reposition_bend
    
    pub fn reoffset_bend(&mut self, bend: BendIndex, offset: f64) {

    }

    pub fn nodes(&self) -> impl Iterator<Item=TaggedIndex> + '_ {
        self.rtree.iter().map(|wrapper| wrapper.data)
    }

    pub fn primitive<Weight>(&self, index: Index<Weight>) -> Primitive<Weight> {
        Primitive::new(index, &self.graph)
    }

    fn insert_into_rtree(&mut self, index: TaggedIndex) {
        let shape = untag!(index, self.primitive(index).shape());
        self.rtree.insert(RTreeWrapper::new(shape, index));
    }

    fn remove_from_rtree(&mut self, index: TaggedIndex) {
        let shape = untag!(index, self.primitive(index).shape());
        self.rtree.remove(&RTreeWrapper::new(shape, index));
    }
}
