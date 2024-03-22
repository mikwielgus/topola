use petgraph::stable_graph::StableGraph;

use crate::layout::{graph::GeometryIndex, groups::GetGroups};

#[derive(Debug)]
pub struct DsnGroups {
    map: BTreeMap<
}

impl GetGroups for DsnGroups {
    fn node_groups(&self, node: GeometryIndex) -> Vec<GroupIndex> {
        //
    }
}
