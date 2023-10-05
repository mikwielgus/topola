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
    pub fn new(
        from: DotIndex,
        to: DotIndex,
        graph: &StableDiGraph<TaggedWeight, Label, usize>,
    ) -> Self {
        let mut this = Self {
            from,
            to,
            interior: vec![],
        };
        let mut prev_index = from.tag();

        while let Some(index) = untag!(prev_index, Primitive::new(prev_index, graph).tagged_next())
        {
            this.interior.push(index);
            prev_index = index;
        }

        this
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
/*use petgraph::stable_graph::StableDiGraph;

use crate::{
    graph::{DotIndex, Ends, Interior, Label, Tag, TaggedIndex, TaggedWeight},
    primitive::Primitive,
};

pub struct Band {
    from: DotIndex,
    to: DotIndex,
}

impl Band {
    pub fn new(from: DotIndex, to: DotIndex) -> Self {
        Self { from, to }
    }
}

struct BandIterator<'a> {
    graph: &'a StableDiGraph<TaggedWeight, Label, usize>,
    index: TaggedIndex,
}

impl<'a> Iterator for BandIterator<'a> {
    type Item = TaggedIndex;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        let next_index = untag!(index, Primitive::new(index, self.graph).tagged_next());

        if let Some(next_index) = next_index {
            self.index = next_index;
        }

        next_index
    }
}

impl<'a> Interior<'a, TaggedIndex> for Band {
    type Iter = BandIterator<'a>;

    fn interior(&self, graph: &'a StableDiGraph<TaggedWeight, Label, usize>) -> Self::Iter {
        BandIterator {
            graph,
            index: self.from.tag(),
        }
    }
}

impl Ends<DotIndex, DotIndex> for Band {
    fn ends(&self) -> (DotIndex, DotIndex) {
        (self.from, self.to)
    }
}*/
