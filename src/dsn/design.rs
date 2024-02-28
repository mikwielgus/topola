use std::collections::HashMap;

use crate::{
    layout::{dot::FixedDotWeight, seg::FixedSegWeight, Layout},
    math::Circle,
};

use super::{
    de::{from_str, Error},
    structure::Pcb,
    rules::Rules,
};

#[derive(Debug)]
pub struct DsnDesign {
    pcb: Pcb,
    rules: Rules,
}

impl DsnDesign {
    pub fn load_from_file(filename: &str) -> Result<Self, Error> {
        let contents = std::fs::read_to_string(filename).unwrap(); // TODO: remove unwrap.
        let pcb = from_str::<Pcb>(&contents)?;

        let rules = Rules::from_pcb(&pcb);

        Ok(Self {
            pcb,
            rules,
        })
    }

    pub fn make_layout(&self) -> Layout<&Rules> {
        let mut layout = Layout::new(&self.rules);

        // mapping of pin id -> net id prepared for adding pins
        let pin_nets = if let Some(nets) = self.pcb.network.nets.as_ref() {
            HashMap::<String, i64>::from_iter(
                nets.iter()
                    .map(|net| {
                        // resolve the id so we don't work with strings
                        let net_id = self.rules.net_ids.get(&net.name).unwrap();

                        // take the list of pins
                        // and for each pin id output (pin id, net id)
                        net.pins.ids.iter().map(|id| (id.clone(), *net_id))
                    })
                    // flatten the nested iters into a single stream of tuples
                    .flatten(),
            )
        } else {
            HashMap::<String, i64>::new()
        };

        // add pins from components
        //self.pcb.placement.components.iter().map(|component| {
        for component in &self.pcb.placement.components {
            for place in &component.places {
                let image = self
                    .pcb
                    .library
                    .images
                    .iter()
                    .find(|image| image.name == component.name)
                    .unwrap();

                for pin in &image.pins {
                    let pin_name = format!("{}-{}", place.name, pin.id);
                    let net_id = pin_nets.get(&pin_name).unwrap();
                    let continent = layout.add_continent(*net_id);

                    let padstack = &self
                        .pcb
                        .library
                        .padstacks
                        .iter()
                        .find(|padstack| padstack.name == pin.name)
                        .unwrap();

                    // no layer support yet, pick the first one
                    let circle = &padstack.shapes[0].0;
                    let circle = Circle {
                        pos: (place.x as f64 / 100.0, -place.y as f64 / 100.0).into(),
                        r: circle.diameter as f64 / 200.0,
                    };

                    layout
                        .add_fixed_dot(FixedDotWeight { continent, circle })
                        .unwrap();
                }
            }
        }
        //})

        // add vias to layout and save indices of dots in the order they appear in the file
        let _dot_indices: Vec<_> = self
            .pcb
            .wiring
            .vias
            .iter()
            .map(|via| {
                let net_id = self.rules.net_ids.get(&via.net.0).unwrap();
                let continent = layout.add_continent(*net_id);

                // find the padstack referenced by this via placement
                let padstack = &self
                    .pcb
                    .library
                    .padstacks
                    .iter()
                    .find(|padstack| padstack.name == via.name)
                    .unwrap();

                // no layer support yet, pick the first one
                let circle = &padstack.shapes[0].0;
                let circle = Circle {
                    pos: (via.x as f64 / 100.0, -via.y as f64 / 100.0).into(),
                    r: circle.diameter as f64 / 200.0,
                };

                layout
                    .add_fixed_dot(FixedDotWeight { continent, circle })
                    .unwrap()
            })
            .collect();

        for wire in self.pcb.wiring.wires.iter() {
            let net_id = self.rules.net_ids.get(&wire.net.0).unwrap();
            let continent = layout.add_continent(*net_id);

            // add the first coordinate in the wire path as a dot and save its index
            let mut prev_index = layout
                .add_fixed_dot(FixedDotWeight {
                    continent,
                    circle: Circle {
                        pos: (
                            wire.path.coords[0].x as f64 / 100.0,
                            -wire.path.coords[0].y as f64 / 100.0,
                        )
                            .into(),
                        r: wire.path.width as f64 / 100.0,
                    },
                })
                .unwrap();

            // iterate through path coords starting from the second
            for coord in wire.path.coords.iter().skip(1) {
                let index = layout
                    .add_fixed_dot(FixedDotWeight {
                        continent,
                        circle: Circle {
                            pos: (coord.x as f64 / 100.0, -coord.y as f64 / 100.0).into(),
                            r: wire.path.width as f64 / 100.0,
                        },
                    })
                    .unwrap();

                // add a seg between the current and previous coords
                let _ = layout
                    .add_fixed_seg(
                        prev_index,
                        index,
                        FixedSegWeight {
                            continent,
                            width: wire.path.width as f64 / 100.0,
                        },
                    )
                    .unwrap();

                prev_index = index;
            }
        }

        layout
    }
}
