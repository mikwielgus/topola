#[derive(Debug, Default)]
pub struct Conditions {
    layer: Option<String>,
    region: Option<String>,
    netclass: Option<String>,
}

#[derive(Debug, Default)]
pub struct LayerNetclassConditions {
    region: Option<String>,
}

pub trait RulesTrait {
    fn clearance(&self, conditions1: &Conditions, conditions2: &Conditions) -> f64;
    fn clearance_limit(
        &self,
        layer: String,
        netclass: String,
        conditions: &LayerNetclassConditions,
    ) -> f64;
}
