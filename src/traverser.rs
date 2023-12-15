use petgraph::stable_graph::StableDiGraph;

use crate::{
    graph::{Label, LooseBendIndex, Weight},
    layout::Layout,
    primitive::{GenericPrimitive, GetInnerOuter},
};

pub struct OutwardRailTraverser<'a> {
    rail: Option<LooseBendIndex>,
    layout: &'a Layout,
}

impl<'a> OutwardRailTraverser<'a> {
    pub fn new(rail: Option<LooseBendIndex>, layout: &'a Layout) -> Self {
        Self { rail, layout }
    }
}

impl<'a> Iterator for OutwardRailTraverser<'a> {
    type Item = LooseBendIndex;

    fn next(&mut self) -> Option<Self::Item> {
        self.rail.map(|rail| {
            self.rail = GenericPrimitive::new(rail, self.layout).outer();
            rail
        })
    }
}
