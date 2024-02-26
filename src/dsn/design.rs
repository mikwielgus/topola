use std::collections::HashMap;

use crate::{
    layout::{
        dot::{DotGeodata, FixedDotWeight},
        seg::{FixedSegWeight, SegGeodata},
        Layout,
    },
    math::Circle,
};

use super::{
    de::{from_str, Error},
    structure::Pcb,
};

#[derive(Debug)]
pub struct DsnDesign {
    pcb: Pcb,
}

impl DsnDesign {
    pub fn load_from_file(filename: &str) -> Result<Self, Error> {
        let contents = std::fs::read_to_string(filename).unwrap(); // TODO: remove unwrap.

        Ok(Self {
            pcb: from_str::<Pcb>(&contents)?,
        })
    }

    pub fn make_layout(&self) -> Layout<&Pcb> {
        let mut layout = Layout::new(&self.pcb);

        // this holds the mapping of net names to numerical IDs (here for now)
        let net_ids: HashMap<String, usize> = HashMap::from_iter(
            self.pcb.network.classes[0]
                .nets
                .iter()
                .enumerate()
                .map(|(id, net)| (net.clone(), id)),
        );

        let continent100 = layout.add_continent(100); // TODO: remove this placeholder.

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
                    //let pin_name = format!("{}-{}", place.name, pin.id);
                    /*let net = self
                        .pcb
                        .network
                        .nets
                        .unwrap()
                        .find(|net| net.pins[0].ids.contains(&pin_name));
                    let net_id = net_ids.get(&net).unwrap();
                    let continent = layout.add_continent(*net_id as i64);*/
                    //let continent = layout.add_continent(*net_id as i64);

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
                        .add_fixed_dot(FixedDotWeight {
                            continent: continent100,
                            geodata: DotGeodata { circle },
                        })
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
                let net_id = net_ids.get(&via.net.0).unwrap();
                let continent = layout.add_continent(*net_id as i64);

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
                    .add_fixed_dot(FixedDotWeight {
                        continent,
                        geodata: DotGeodata { circle },
                    })
                    .unwrap()
            })
            .collect();

        for wire in self.pcb.wiring.wires.iter() {
            let net_id = net_ids.get(&wire.net.0).unwrap();
            let continent = layout.add_continent(*net_id as i64);

            // add the first coordinate in the wire path as a dot and save its index
            let mut prev_index = layout
                .add_fixed_dot(FixedDotWeight {
                    continent,
                    geodata: DotGeodata {
                        circle: Circle {
                            pos: (
                                wire.path.coords[0].x as f64 / 100.0,
                                -wire.path.coords[0].y as f64 / 100.0,
                            )
                                .into(),
                            r: wire.path.width as f64 / 100.0,
                        },
                    },
                })
                .unwrap();

            // iterate through path coords starting from the second
            for coord in wire.path.coords.iter().skip(1) {
                let index = layout
                    .add_fixed_dot(FixedDotWeight {
                        continent,
                        geodata: DotGeodata {
                            circle: Circle {
                                pos: (coord.x as f64 / 100.0, -coord.y as f64 / 100.0).into(),
                                r: wire.path.width as f64 / 100.0,
                            },
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
                            geodata: SegGeodata {
                                width: wire.path.width as f64 / 100.0,
                            },
                        },
                    )
                    .unwrap();

                prev_index = index;
            }
        }

        layout
    }
}
