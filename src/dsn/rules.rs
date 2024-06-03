use std::collections::HashMap;

use crate::{
    drawing::rules::{Conditions, RulesTrait},
    dsn::structure::Pcb,
};

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

    // layernames -> layers for Layout
    pub layername_to_layer: HashMap<String, u64>,
    // netnames -> nets for Layout
    pub netname_to_net: HashMap<String, usize>,
    // net -> netclass
    net_to_netclass: HashMap<usize, String>,
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
        let netname_to_net = HashMap::from_iter(
            pcb.network
                .class_vec
                .iter()
                .flat_map(|class| &class.net_vec)
                .enumerate()
                .map(|(id, net)| (net.clone(), id)),
        );

        let mut net_id_classes = HashMap::new();
        let class_rules = HashMap::from_iter(
            pcb.network
                .class_vec
                .iter()
                .inspect(|class| {
                    for net in &class.net_vec {
                        let net_id = netname_to_net.get(net).unwrap();
                        net_id_classes.insert(*net_id, class.name.clone());
                    }
                })
                .map(|class| (class.name.clone(), DsnRule::from_dsn(&class.rule))),
        );

        Self {
            structure_rule: DsnRule::from_dsn(&pcb.structure.rule),
            class_rules,
            layername_to_layer: layer_ids,
            netname_to_net,
            net_to_netclass: net_id_classes,
        }
    }

    pub fn get_rule(&self, net: usize) -> &DsnRule {
        if let Some(netclass) = self.net_to_netclass.get(&net) {
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
        let (Some(net1), Some(net2)) = (conditions1.maybe_net, conditions2.maybe_net) else {
            return 0.0;
        };

        let clr1 = self.get_rule(net1).clearance;
        let clr2 = self.get_rule(net2).clearance;

        if clr1 > clr2 {
            clr1
        } else {
            clr2
        }
    }

    fn largest_clearance(&self, maybe_net: Option<usize>) -> f64 {
        let mut largest: f64 = 0.0;

        for (class, rule) in &self.class_rules {
            if rule.clearance > largest {
                largest = rule.clearance;
            }
        }

        largest
    }
}
