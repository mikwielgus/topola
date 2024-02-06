use enum_dispatch::enum_dispatch;

use crate::layout::primitive::Primitive;

#[enum_dispatch]
pub trait GetConditions {
    fn conditions(&self) -> Conditions;
}

#[derive(Debug, Default)]
pub struct Conditions {
    pub net: i64,
    pub region: Option<String>,
    pub layer: Option<String>,
}

pub trait RulesTrait {
    fn clearance(&self, conditions1: &Conditions, conditions2: &Conditions) -> f64;
    /*fn clearance_limit(
        &self,
        layer: String,
        netclass: String,
        conditions: &PrimitiveConditions,
    ) -> f64;*/
}