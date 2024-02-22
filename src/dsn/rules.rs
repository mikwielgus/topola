use crate::layout::rules::{Conditions, RulesTrait};

use super::structure::Pcb;

impl<'a> RulesTrait for &'a Pcb {
    fn clearance(&self, _conditions1: &Conditions, _conditions2: &Conditions) -> f64 {
        // Placeholder for now.
        10.0
    }

    fn clearance_net_limit(&self, _net: i64) -> f64 {
        // Placeholder for now.
        10.0
    }
}
