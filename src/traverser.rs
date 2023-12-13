use petgraph::stable_graph::StableDiGraph;

use crate::{
    graph::{Label, LooseBendIndex, Weight},
    primitive::{GenericPrimitive, GetInnerOuter},
};

pub struct OutwardRailTraverser<'a> {
    rail: Option<LooseBendIndex>,
    graph: &'a StableDiGraph<Weight, Label, usize>,
}

impl<'a> OutwardRailTraverser<'a> {
    pub fn new(
        rail: Option<LooseBendIndex>,
        graph: &'a StableDiGraph<Weight, Label, usize>,
    ) -> Self {
        Self { rail, graph }
    }
}

impl<'a> Iterator for OutwardRailTraverser<'a> {
    type Item = LooseBendIndex;

    fn next(&mut self) -> Option<Self::Item> {
        self.rail.map(|rail| {
            self.rail = GenericPrimitive::new(rail, self.graph).outer();
            rail
        })
    }
}
