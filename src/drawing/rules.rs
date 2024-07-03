use enum_dispatch::enum_dispatch;

use crate::drawing::primitive::Primitive;

#[enum_dispatch]
pub trait GetConditions {
    fn conditions(&self) -> Conditions;
}

#[derive(Debug, Default)]
pub struct Conditions {
    pub maybe_net: Option<usize>,
    pub maybe_region: Option<String>,
    pub maybe_layer: Option<String>,
}

pub trait AccessRules {
    fn clearance(&self, conditions1: &Conditions, conditions2: &Conditions) -> f64;
    fn largest_clearance(&self, net: Option<usize>) -> f64;
}
