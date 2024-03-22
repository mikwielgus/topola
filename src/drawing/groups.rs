use enum_dispatch::enum_dispatch;

use super::graph::GeometryIndex;

#[enum_dispatch]
pub trait GetGroups< {
    fn node_groups(&self, node: GeometryIndex) -> Vec<GeometryIndex>;
}
