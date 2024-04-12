use crate::graph::{GenericIndex, GetNodeIndex};

pub trait GroupingManagerTrait<GW: Copy, GI: GetNodeIndex + Copy> {
    fn add_grouping(&mut self, weight: GW) -> GenericIndex<GW>;
    fn remove_grouping(&mut self, grouping: GenericIndex<GW>);
    fn assign_to_grouping<W>(&mut self, node: GenericIndex<W>, grouping: GenericIndex<GW>);
    fn groupings<W>(&self, node: GenericIndex<W>) -> impl Iterator<Item = GenericIndex<GW>>;
}
