use std::collections::HashMap;

use bimap::BiHashMap;

use crate::{
    board::mesadata::AccessMesadata,
    drawing::rules::{AccessRules, Conditions},
    specctra::structure::Pcb,
};

#[derive(Debug)]
/// [`SpecctraRule`] represents the basic routing constraints used by an auto-router, such as
/// the Specctra auto-router, in a PCB design process. This struct defines two key design
/// rules: the width of the trace and the minimum clearance between electrical features.
pub struct SpecctraRule {
    /// Specifies the width of the trace (or conductor) in millimeters. 
    /// This value ensures that the traces meet electrical
    /// and mechanical requirements, such as current-carrying capacity or signal integrity.
    pub width: f64,
    /// Defines the minimum clearance (spacing) between traces, pads,
    /// or other conductive features on the PCB. Adequate clearance is important for
    /// preventing electrical shorts or interference between signals, and is often
    /// dictated by manufacturing constraints or voltage considerations.
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
/// [`SpecctraMesadata`] holds the metadata required by the Specctra auto-router to
/// understand and enforce design rules across various net classes and layers in a PCB layout.
/// This struct encapsulates information about rules for individual nets, net classes,
/// layers, and their corresponding relationships.
pub struct SpecctraMesadata {
    
    /// The default routing rule applied globally if no specific net class rule is defined.
    structure_rule: SpecctraRule,
    
    // net class name -> rule
    /// A map from net class names to their specific `SpecctraRule` constraints.
    /// These rules are applied to all nets belonging to the respective net clas
    class_rules: HashMap<String, SpecctraRule>,

    // layername <-> layer for Layout
    /// A bidirectional map between layer indices and layer names, allowing translation
    /// between index-based layers in the layout and user-defined layer names.
    pub layer_layername: BiHashMap<usize, String>,

    // netname <-> net for Layout
    /// A bidirectional map between network indices and network names in the PCB layout,
    /// providing an easy way to reference nets by name or index.
    pub net_netname: BiHashMap<usize, String>,

    // net -> netclass
    /// A map that associates network indices with their respective net class names.
    /// This is used to apply net class-specific routing rules to each net.
    net_netclass: HashMap<usize, String>,
}


impl SpecctraMesadata {
    /// Creates a [`SpecctraMesadata`] instance from a given `Pcb` reference.
    /// 
    /// This function extracts the necessary metadata from the `Pcb` struct, such as
    /// layer-to-layer name mappings, net-to-net name mappings, and net class rules.
    ///
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

        let mut structure_rule = super::structure::Rule {
            width: 0.0,
            clearances: Vec::new(),
        };
        // workaround for differing syntax
        // collapse multiple rule entries into a single one
        for rule in &pcb.structure.rules {
            if rule.width.is_some() {
                structure_rule.width = rule.width.unwrap()
            }
            structure_rule.clearances.extend_from_slice(&rule.clearances);
        }

        Self {
            structure_rule: SpecctraRule::from_dsn(&structure_rule),
            class_rules,
            layer_layername,
            net_netname,
            net_netclass,
        }
    }

    /// Retrieves the Specctra routing rule associated with a specified net ID.
    ///
    /// This function looks up the routing rule for a given net ID. It first checks if the net is 
    /// associated with a net class. If a net class is found, it retrieves the corresponding rule 
    /// from the class rules. If no class is associated, or if the class does not have a defined rule, 
    /// it defaults to the general structure rule.
    ///
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
