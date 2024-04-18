use crate::graph::{GenericIndex, GetNodeIndex};

pub trait CompoundManagerTrait<CW: Copy, GI: GetNodeIndex + Copy> {
    fn add_compound(&mut self, weight: CW) -> GenericIndex<CW>;
    fn remove_compound(&mut self, compound: GenericIndex<CW>);
    fn add_to_compound<W>(&mut self, node: GenericIndex<W>, compound: GenericIndex<CW>);
    fn compound_weight(&self, node: GenericIndex<CW>) -> CW;
    fn compounds<W>(&self, node: GenericIndex<W>) -> impl Iterator<Item = GenericIndex<CW>>;
}
