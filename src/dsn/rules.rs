use std::collections::HashMap;

use crate::drawing::rules::{Conditions, RulesTrait};

use super::structure::Pcb;

#[derive(Debug)]
pub struct DsnRule {
    pub width: f64,
    pub clearance: f64,
}

impl DsnRule {
    fn from_dsn(rule: &super::structure::Rule) -> Self {
        Self {
            width: rule.width as f64 / 100.0,
            clearance: rule.clearance_vec[0].value as f64 / 100.0, // picks the generic clearance only for now
        }
    }
}

#[derive(Debug)]
pub struct DsnRules {
    structure_rule: DsnRule,
    // net class name -> rule
    class_rules: HashMap<String, DsnRule>,

    // layer names -> layer IDs for Layout
    pub layer_ids: HashMap<String, u64>,
    // net names -> net IDs for Layout
    pub net_ids: HashMap<String, i64>,
    // net ID -> net class
    net_id_classes: HashMap<i64, String>,
}

impl DsnRules {
    pub fn from_pcb(pcb: &Pcb) -> Self {
        let layer_ids = HashMap::from_iter(
            pcb.structure
                .layer_vec
                .iter()
                .map(|layer| (layer.name.clone(), layer.property.index as u64)),
        );

        // keeping this as a separate iter pass because it might be moved into a different struct later?
        let net_ids = HashMap::from_iter(
            pcb.network
                .class_vec
                .iter()
                .flat_map(|class| &class.net_vec)
                .enumerate()
                .map(|(id, net)| (net.clone(), id as i64)),
        );

        let mut net_id_classes = HashMap::new();
        let class_rules = HashMap::from_iter(
            pcb.network
                .class_vec
                .iter()
                .inspect(|class| {
                    for net in &class.net_vec {
                        let net_id = net_ids.get(net).unwrap();
                        net_id_classes.insert(*net_id, class.name.clone());
                    }
                })
                .map(|class| (class.name.clone(), DsnRule::from_dsn(&class.rule))),
        );

        Self {
            structure_rule: DsnRule::from_dsn(&pcb.structure.rule),
            class_rules,
            layer_ids,
            net_ids,
            net_id_classes,
        }
    }

    pub fn get_rule(&self, net: i64) -> &DsnRule {
        if let Some(netclass) = self.net_id_classes.get(&net) {
            self.class_rules
                .get(netclass)
                .unwrap_or(&self.structure_rule)
        } else {
            &self.structure_rule
        }
    }
}

impl RulesTrait for DsnRules {
    fn clearance(&self, conditions1: &Conditions, conditions2: &Conditions) -> f64 {
        let clr1 = self.get_rule(conditions1.net).clearance;
        let clr2 = self.get_rule(conditions2.net).clearance;

        if clr1 > clr2 {
            clr1
        } else {
            clr2
        }
    }

    fn largest_clearance(&self, _net: i64) -> f64 {
        let mut largest: f64 = 0.0;

        for (class, rule) in &self.class_rules {
            if rule.clearance > largest {
                largest = rule.clearance;
            }
        }

        largest
    }
}
