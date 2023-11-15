use petgraph::stable_graph::StableDiGraph;

use crate::{
    graph::{Label, LooseBendIndex, Weight},
    primitive::{GenericPrimitive, GetInnerOuter},
};

pub struct OutwardLayerTraverser<'a> {
    layer: Option<LooseBendIndex>,
    graph: &'a StableDiGraph<Weight, Label, usize>,
}

impl<'a> OutwardLayerTraverser<'a> {
    pub fn new(
        layer: Option<LooseBendIndex>,
        graph: &'a StableDiGraph<Weight, Label, usize>,
    ) -> Self {
        Self { layer, graph }
    }
}

impl<'a> Iterator for OutwardLayerTraverser<'a> {
    type Item = LooseBendIndex;

    fn next(&mut self) -> Option<Self::Item> {
        self.layer.map(|layer| {
            self.layer = GenericPrimitive::new(layer, self.graph).outer();
            layer
        })
    }
}
