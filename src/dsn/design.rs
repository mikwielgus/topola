use std::collections::HashMap;

use crate::{
    layout::{dot::FixedDotWeight, seg::FixedSegWeight, Layout},
    math::Circle,
};

use super::{
    de::{from_str, Error},
    structure::Pcb,
};

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

        // add vias to layout and save indices of dots in the order they appear in the file
        let _dot_indices: Vec<_> = self
            .pcb
            .wiring
            .vias
            .iter()
            .map(|via| {
                let net_id = net_ids.get(&via.net.0).unwrap();
                let component = layout.add_component(*net_id as i64);

                // no way to resolve the name or layer support yet
                // pick the first layer of the first object found
                let circle = &self.pcb.library.padstacks[0].shapes[0].0;
                let circle = Circle {
                    pos: (via.x as f64 / 100.0, -via.y as f64 / 100.0).into(),
                    r: circle.radius as f64 / 100.0,
                };

                layout
                    .add_fixed_dot(FixedDotWeight { component, circle })
                    .unwrap()
            })
            .collect();

        for wire in self.pcb.wiring.wires.iter() {
            let net_id = net_ids.get(&wire.net.0).unwrap();
            let component = layout.add_component(*net_id as i64);

            // add the first coordinate in the wire path as a dot and save its index
            let mut prev_index = layout
                .add_fixed_dot(FixedDotWeight {
                    component,
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
                        component,
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
                            component,
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
