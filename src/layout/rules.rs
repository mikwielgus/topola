pub struct Conditions {
    layer: Option<String>,
    region: Option<String>,
    netclass: Option<String>,
}

pub struct LayerNetclassConditions {
    region: Option<String>,
}

pub trait RulesTrait {
    fn clearance(conditions1: &Conditions, conditions2: &Conditions) -> f64;
    fn clearance_limit(
        layer: String,
        netclass: String,
        conditions: &LayerNetclassConditions,
    ) -> f64;
}
