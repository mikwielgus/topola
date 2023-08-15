use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Conditions {
    pub lower_net: Option<i32>,
    pub higher_net: Option<i32>,
    pub layer: Option<i32>,
    pub zone: Option<i32>,
}

impl Conditions {
    pub fn priority(&self) -> i32 {
        let mut priority = 0;
        priority += (self.lower_net.is_some() as i32) * 1;
        priority += (self.higher_net.is_some() as i32) * 2;
        priority += (self.layer.is_some() as i32) * 4;
        priority += (self.zone.is_some() as i32) * 8;
        priority
    }
}

pub struct Rules {
    rulesets: [Option<HashMap<Conditions, Ruleset>>; 16],
}

impl Rules {
    pub fn new() -> Self {
        let mut me = Self {
            rulesets: Default::default(),
        };
        me.rulesets[0] = Some(HashMap::from([(
            Conditions {
                lower_net: None,
                higher_net: None,
                layer: None,
                zone: None,
            },
            Ruleset::new(),
        )]));
        me
    }

    pub fn ruleset(&self, conditions: Conditions) -> &Ruleset {
        let priority = conditions.priority();

        for index in (1..(priority + 1)).rev() {
            if let Some(ruleset_hashmap) = &self.rulesets[index as usize] {
                if let Some(ruleset) = ruleset_hashmap.get(&conditions) {
                    return ruleset;
                }
            }
        }

        &self.rulesets[0].as_ref().unwrap()[&conditions]
    }
}

pub struct Ruleset {
    pub length: Rule,
    pub clearance: Rule,
}

impl Ruleset {
    pub fn new() -> Self {
        Self {
            length: Rule::new(),
            clearance: Rule::new(),
        }
    }
}

pub struct Rule {
    pub min: f64,
    pub opt: Option<f64>,
    pub max: f64,
}

impl Rule {
    pub fn new() -> Self {
        Self {
            min: 0.0,
            opt: None,
            max: f64::INFINITY,
        }
    }
}
