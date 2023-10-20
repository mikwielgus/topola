use petgraph::stable_graph::StableDiGraph;

use crate::{
    graph::{DotIndex, Ends, Index, Interior, Label, MakePrimitive, Weight},
    primitive::TaggedPrevTaggedNext,
};

pub struct Band {
    from: DotIndex,
    to: DotIndex,
    interior: Vec<Index>,
}

impl Band {
    pub fn from_dot_prev(
        dot: DotIndex,
        graph: &StableDiGraph<Weight, Label, usize>,
    ) -> Option<Self> {
        let mut next_index = Index::Dot(dot);
        let mut interior = vec![];

        while let Some(index) = next_index.primitive(graph).tagged_prev() {
            interior.push(index);
            next_index = index;
        }

        if interior.is_empty() {
            None
        } else {
            Some(Self {
                from: interior.pop().unwrap().into_dot().unwrap(),
                to: dot,
                interior,
            })
        }
    }

    pub fn from_dot_next(
        dot: DotIndex,
        graph: &StableDiGraph<Weight, Label, usize>,
    ) -> Option<Self> {
        let mut prev_index = Index::Dot(dot);
        let mut interior = vec![];

        while let Some(index) = prev_index.primitive(graph).tagged_next() {
            interior.push(index);
            prev_index = index;
        }

        if interior.is_empty() {
            None
        } else {
            Some(Self {
                from: dot,
                to: interior.pop().unwrap().into_dot().unwrap(),
                interior,
            })
        }
    }
}

impl Interior<Index> for Band {
    fn interior(&self) -> Vec<Index> {
        // FIXME: Unnecessary clone. There should be a better way to do it.
        self.interior.clone()
    }
}

impl Ends<DotIndex, DotIndex> for Band {
    fn ends(&self) -> (DotIndex, DotIndex) {
        (self.from, self.to)
    }
}
