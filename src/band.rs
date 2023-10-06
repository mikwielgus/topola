use petgraph::stable_graph::StableDiGraph;

use crate::{
    graph::{DotIndex, Ends, Interior, Label, Tag, TaggedIndex, TaggedWeight},
    primitive::Primitive,
};

pub struct Band {
    from: DotIndex,
    to: DotIndex,
    interior: Vec<TaggedIndex>,
}

impl Band {
    pub fn from_dot_prev(
        dot: DotIndex,
        graph: &StableDiGraph<TaggedWeight, Label, usize>,
    ) -> Option<Self> {
        let mut next_index = dot.tag();
        let mut interior = vec![];

        while let Some(index) = untag!(next_index, Primitive::new(next_index, graph).tagged_next())
        {
            interior.push(index);
            next_index = index;
        }

        if interior.is_empty() {
            None
        } else {
            Some(Self {
                from: *interior.pop().unwrap().as_dot().unwrap(),
                to: dot,
                interior,
            })
        }
    }

    pub fn from_dot_next(
        dot: DotIndex,
        graph: &StableDiGraph<TaggedWeight, Label, usize>,
    ) -> Option<Self> {
        let mut prev_index = dot.tag();
        let mut interior = vec![];

        while let Some(index) = untag!(prev_index, Primitive::new(prev_index, graph).tagged_next())
        {
            interior.push(index);
            prev_index = index;
        }

        if interior.is_empty() {
            None
        } else {
            Some(Self {
                from: dot,
                to: *interior.pop().unwrap().as_dot().unwrap(),
                interior,
            })
        }
    }
}

impl Interior<TaggedIndex> for Band {
    fn interior(&self) -> Vec<TaggedIndex> {
        // FIXME: Unnecessary clone. There should be a better way to do it.
        self.interior.clone()
    }
}

impl Ends<DotIndex, DotIndex> for Band {
    fn ends(&self) -> (DotIndex, DotIndex) {
        (self.from, self.to)
    }
}
