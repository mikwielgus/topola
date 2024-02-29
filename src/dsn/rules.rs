use std::collections::HashMap;

use crate::layout::rules::{Conditions, RulesTrait};

use super::structure::Pcb;

#[derive(Debug)]
pub struct Rule {
    pub width: f64,
    pub clearance: f64,
}

impl Rule {
    fn from_dsn(rule: &super::structure::Rule) -> Self {
        Self {
            width: rule.width.0 as f64 / 100.0,
            clearance: rule.clearances[0].value as f64 / 100.0, // picks the generic clearance only for now
        }
    }
}

#[derive(Debug)]
pub struct Rules {
    structure_rule: Rule,
    // net class name -> rule
    class_rules: HashMap<String, Rule>,

    // net names -> net IDs for Layout
    pub net_ids: HashMap<String, i64>,
    // net ID -> net class
    net_id_classes: HashMap<i64, String>,
}

impl Rules {
    pub fn from_pcb(pcb: &Pcb) -> Self {
        // keeping this as a separate iter pass because it might be moved into a different struct later?
        let net_ids = HashMap::from_iter(
            pcb.network.classes
                .iter()
                .flat_map(|class| &class.nets)
                .enumerate()
                .map(|(id, net)| (net.clone(), id as i64)),
        );

        let mut net_id_classes = HashMap::new();
        let class_rules = HashMap::from_iter(
            pcb.network
                .classes
                .iter()
                .inspect(|class| {
                    for net in &class.nets {
                        let net_id = net_ids.get(net).unwrap();
                        net_id_classes.insert(*net_id, class.name.clone());
                    }
                })
                .map(|class| (class.name.clone(), Rule::from_dsn(&class.rule))),
        );

        Self {
            structure_rule: Rule::from_dsn(&pcb.structure.rule),
            class_rules,
            net_ids,
            net_id_classes,
        }
    }

    pub fn get_rule(&self, net: i64) -> &Rule {
        if let Some(netclass) = self.net_id_classes.get(&net) {
            self.class_rules
                .get(netclass)
                .unwrap_or(&self.structure_rule)
        } else {
            &self.structure_rule
        }
    }
}

impl<'a> RulesTrait for &'a Rules {
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
