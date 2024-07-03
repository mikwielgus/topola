use std::collections::HashMap;

use bimap::BiHashMap;

use crate::{
    board::mesadata::AccessMesadata,
    drawing::rules::{AccessRules, Conditions},
    specctra::structure::Pcb,
};

#[derive(Debug)]
pub struct SpecctraRule {
    pub width: f64,
    pub clearance: f64,
}

impl SpecctraRule {
    fn from_dsn(rule: &super::structure::Rule) -> Self {
        Self {
            width: rule.width as f64,
            clearance: rule.clearances[0].value as f64, // picks the generic clearance only for now
        }
    }
}

#[derive(Debug)]
pub struct SpecctraMesadata {
    structure_rule: SpecctraRule,
    // net class name -> rule
    class_rules: HashMap<String, SpecctraRule>,

    // layername <-> layer for Layout
    pub layer_layername: BiHashMap<usize, String>,

    // netname <-> net for Layout
    pub net_netname: BiHashMap<usize, String>,

    // net -> netclass
    net_netclass: HashMap<usize, String>,
}

impl SpecctraMesadata {
    pub fn from_pcb(pcb: &Pcb) -> Self {
        let layer_layername = BiHashMap::from_iter(
            pcb.structure
                .layers
                .iter()
                .map(|layer| (layer.property.index, layer.name.clone())),
        );

        // keeping this as a separate iter pass because it might be moved into a different struct later?
        let net_netname = BiHashMap::from_iter(
            pcb.network
                .classes
                .iter()
                .flat_map(|class| &class.nets)
                .enumerate()
                .map(|(net, netname)| (net, netname.clone())),
        );

        let mut net_netclass = HashMap::new();
        let class_rules = HashMap::from_iter(
            pcb.network
                .classes
                .iter()
                .inspect(|class| {
                    for netname in &class.nets {
                        let net = net_netname.get_by_right(netname).unwrap();
                        net_netclass.insert(*net, class.name.clone());
                    }
                })
                .map(|class| (class.name.clone(), SpecctraRule::from_dsn(&class.rule))),
        );

        Self {
            structure_rule: SpecctraRule::from_dsn(&pcb.structure.rule),
            class_rules,
            layer_layername,
            net_netname,
            net_netclass,
        }
    }

    pub fn get_rule(&self, net: usize) -> &SpecctraRule {
        if let Some(netclass) = self.net_netclass.get(&net) {
            self.class_rules
                .get(netclass)
                .unwrap_or(&self.structure_rule)
        } else {
            &self.structure_rule
        }
    }
}

impl AccessRules for SpecctraMesadata {
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

impl AccessMesadata for SpecctraMesadata {
    fn bename_layer(&mut self, layer: usize, layername: String) {
        self.layer_layername.insert(layer, layername);
    }

    fn layer_layername(&self, layer: usize) -> Option<&str> {
        self.layer_layername.get_by_left(&layer).map(|s| s.as_str())
    }

    fn layername_layer(&self, layername: &str) -> Option<usize> {
        self.layer_layername.get_by_right(layername).copied()
    }

    fn bename_net(&mut self, net: usize, netname: String) {
        self.net_netname.insert(net, netname);
    }

    fn net_netname(&self, net: usize) -> Option<&str> {
        self.net_netname.get_by_left(&net).map(|s| s.as_str())
    }

    fn netname_net(&self, netname: &str) -> Option<usize> {
        self.net_netname.get_by_right(netname).copied()
    }
}
