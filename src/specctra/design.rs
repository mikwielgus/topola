use std::collections::HashMap;

use geo::{point, Point, Rotate};
use thiserror::Error;

use crate::{
    board::{mesadata::AccessMesadata, Board},
    drawing::{
        dot::FixedDotWeight,
        graph::{GetLayer, GetMaybeNet, MakePrimitive},
        primitive::MakePrimitiveShape,
        seg::FixedSegWeight,
        Drawing,
    },
    geometry::{primitive::PrimitiveShape, GetWidth},
    layout::{poly::SolidPolyWeight, Layout},
    math::Circle,
    specctra::{
        mesadata::SpecctraMesadata,
        read::{self, ListTokenizer},
        structure::{self, DsnFile, Layer, Pcb, Shape},
        write::{self, ListWriter},
    },
};

#[derive(Error, Debug)]
pub enum LoadingError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Parse(#[from] read::ParseError),
}

#[derive(Debug)]
pub struct SpecctraDesign {
    pcb: Pcb,
}

impl SpecctraDesign {
    pub fn load(reader: impl std::io::BufRead) -> Result<SpecctraDesign, LoadingError> {
        let mut list_reader = ListTokenizer::new(reader);
        let dsn = list_reader.read_value::<DsnFile>()?;

        Ok(Self { pcb: dsn.pcb })
    }

    pub fn get_name(&self) -> &str {
        &self.pcb.name
    }

    pub fn write_ses(
        &self,
        board: &Board<SpecctraMesadata>,
        writer: impl std::io::Write,
    ) -> Result<(), std::io::Error> {
        let mesadata = board.mesadata();
        let drawing = board.layout().drawing();

        let mut net_outs = HashMap::<usize, structure::NetOut>::new();
        for index in drawing.primitive_nodes() {
            let primitive = index.primitive(drawing);

            if let Some(net) = primitive.maybe_net() {
                let coords = match primitive.shape() {
                    PrimitiveShape::Seg(seg) => {
                        vec![
                            structure::Point {
                                x: seg.from.x(),
                                y: seg.from.y(),
                            },
                            structure::Point {
                                x: seg.to.x(),
                                y: seg.to.y(),
                            },
                        ]
                    }

                    PrimitiveShape::Bend(bend) => {
                        // Since general circle arcs don't seem to be supported
                        // we're downgrading each one to a chain of straight
                        // line segments.
                        // TODO: make this configurable? pick a smarter value?
                        let segment_count: usize = 100;

                        let circle = bend.circle();
                        let angle_from = bend.start_angle();
                        let angle_step = bend.spanned_angle() / segment_count as f64;

                        let mut points = Vec::new();
                        for i in 0..=segment_count {
                            let x = circle.pos.x()
                                + circle.r * (angle_from + i as f64 * angle_step).cos();
                            let y = circle.pos.y()
                                + circle.r * (angle_from + i as f64 * angle_step).sin();
                            points.push(structure::Point { x, y });
                        }
                        points
                    }

                    // Intentionally skipped for now.
                    // Topola stores trace segments and dots joining them
                    // as separate objects, but the Specctra formats and KiCad
                    // appear to consider them implicit.
                    // TODO: Vias
                    PrimitiveShape::Dot(_) => continue,
                };

                let wire = structure::WireOut {
                    path: structure::Path {
                        layer: mesadata
                            .layer_layername(primitive.layer())
                            .unwrap()
                            .to_owned(),
                        width: primitive.width(),
                        coords,
                    },
                };

                if let Some(net) = net_outs.get_mut(&net) {
                    net.wire.push(wire);
                } else {
                    net_outs.insert(
                        net,
                        structure::NetOut {
                            name: mesadata.net_netname(net).unwrap().to_owned(),
                            wire: vec![wire],
                            via: Vec::new(),
                        },
                    );
                }
            }
        }

        let ses = structure::SesFile {
            session: structure::Session {
                id: "ID".to_string(),
                routes: structure::Routes {
                    resolution: structure::Resolution {
                        unit: "um".into(),
                        value: 1.0,
                    },
                    library_out: structure::Library {
                        images: Vec::new(),
                        padstacks: Vec::new(),
                    },
                    network_out: structure::NetworkOut {
                        net: net_outs.into_values().collect(),
                    },
                },
            },
        };

        ListWriter::new(writer).write_value(&ses)
    }

    pub fn make_board(&self) -> Board<SpecctraMesadata> {
        let mesadata = SpecctraMesadata::from_pcb(&self.pcb);
        let mut board = Board::new(Layout::new(Drawing::new(
            mesadata,
            self.pcb.structure.layers.len(),
        )));

        // mapping of pin -> net prepared for adding pins
        let pin_nets = HashMap::<String, usize>::from_iter(
            self.pcb
                .network
                .nets
                .iter()
                .map(|net_pin_assignments| {
                    // resolve the id so we don't work with strings
                    let net = board
                        .layout()
                        .drawing()
                        .rules()
                        .netname_net(&net_pin_assignments.name)
                        .unwrap();

                    // take the list of pins
                    // and for each pin id output (pin id, net id)
                    net_pin_assignments
                        .pins
                        .names
                        .iter()
                        .map(move |pinname| (pinname.clone(), net))
                })
                // flatten the nested iters into a single stream of tuples
                .flatten(),
        );

        // add pins from components
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
                    let pinname = format!("{}-{}", place.name, pin.id);
                    let net = pin_nets.get(&pinname).unwrap();

                    let padstack = &self
                        .pcb
                        .library
                        .padstacks
                        .iter()
                        .find(|padstack| padstack.name == pin.name)
                        .unwrap();

                    for shape in padstack.shapes.iter() {
                        match shape {
                            Shape::Circle(circle) => {
                                let layer = Self::layer(
                                    &mut board,
                                    &self.pcb.structure.layers,
                                    &circle.layer,
                                    place.side == "front",
                                );
                                Self::add_circle(
                                    &mut board,
                                    (place.x, place.y).into(),
                                    place.rotation,
                                    (pin.x, pin.y).into(),
                                    pin.rotate.unwrap_or(0.0),
                                    circle.diameter / 2.0,
                                    layer,
                                    *net,
                                    Some(pinname.clone()),
                                )
                            }
                            Shape::Rect(rect) => {
                                let layer = Self::layer(
                                    &mut board,
                                    &self.pcb.structure.layers,
                                    &rect.layer,
                                    place.side == "front",
                                );
                                Self::add_rect(
                                    &mut board,
                                    (place.x, place.y).into(),
                                    place.rotation,
                                    (pin.x, pin.y).into(),
                                    pin.rotate.unwrap_or(0.0),
                                    rect.x1,
                                    rect.y1,
                                    rect.x2,
                                    rect.y2,
                                    layer,
                                    *net,
                                    Some(pinname.clone()),
                                )
                            }
                            Shape::Path(path) => {
                                let layer = Self::layer(
                                    &mut board,
                                    &self.pcb.structure.layers,
                                    &path.layer,
                                    place.side == "front",
                                );
                                Self::add_path(
                                    &mut board,
                                    (place.x, place.y).into(),
                                    place.rotation,
                                    (pin.x, pin.y).into(),
                                    pin.rotate.unwrap_or(0.0),
                                    &path.coords,
                                    path.width,
                                    layer,
                                    *net,
                                    Some(pinname.clone()),
                                )
                            }
                            Shape::Polygon(polygon) => {
                                let layer = Self::layer(
                                    &mut board,
                                    &self.pcb.structure.layers,
                                    &polygon.layer,
                                    place.side == "front",
                                );
                                Self::add_polygon(
                                    &mut board,
                                    (place.x, place.y).into(),
                                    place.rotation,
                                    (pin.x, pin.y).into(),
                                    pin.rotate.unwrap_or(0.0),
                                    &polygon.coords,
                                    polygon.width,
                                    layer,
                                    *net,
                                    Some(pinname.clone()),
                                )
                            }
                        };
                    }
                }
            }
        }

        for via in &self.pcb.wiring.vias {
            let net = board
                .layout()
                .drawing()
                .rules()
                .netname_net(&via.net)
                .unwrap();

            // find the padstack referenced by this via placement
            let padstack = &self
                .pcb
                .library
                .padstacks
                .iter()
                .find(|padstack| padstack.name == via.name)
                .unwrap();

            for shape in &padstack.shapes {
                match shape {
                    Shape::Circle(circle) => {
                        let layer = Self::layer(
                            &mut board,
                            &self.pcb.structure.layers,
                            &circle.layer,
                            true,
                        );
                        Self::add_circle(
                            &mut board,
                            (0.0, 0.0).into(),
                            0.0,
                            (0.0, 0.0).into(),
                            0.0,
                            circle.diameter / 2.0,
                            layer,
                            net,
                            None,
                        )
                    }
                    Shape::Rect(rect) => {
                        let layer =
                            Self::layer(&mut board, &self.pcb.structure.layers, &rect.layer, true);
                        Self::add_rect(
                            &mut board,
                            (0.0, 0.0).into(),
                            0.0,
                            (0.0, 0.0).into(),
                            0.0,
                            rect.x1,
                            rect.y1,
                            rect.x2,
                            rect.y2,
                            layer,
                            net,
                            None,
                        )
                    }
                    Shape::Path(path) => {
                        let layer =
                            Self::layer(&mut board, &self.pcb.structure.layers, &path.layer, true);
                        Self::add_path(
                            &mut board,
                            (0.0, 0.0).into(),
                            0.0,
                            (0.0, 0.0).into(),
                            0.0,
                            &path.coords,
                            path.width,
                            layer,
                            net,
                            None,
                        )
                    }
                    Shape::Polygon(polygon) => {
                        let layer = Self::layer(
                            &mut board,
                            &self.pcb.structure.layers,
                            &polygon.layer,
                            true,
                        );
                        Self::add_polygon(
                            &mut board,
                            (0.0, 0.0).into(),
                            0.0,
                            (0.0, 0.0).into(),
                            0.0,
                            &polygon.coords,
                            polygon.width,
                            layer,
                            net,
                            None,
                        )
                    }
                };
            }
        }

        for wire in self.pcb.wiring.wires.iter() {
            let layer = board
                .layout()
                .drawing()
                .rules()
                .layername_layer(&wire.path.layer)
                .unwrap();
            let net = board
                .layout()
                .drawing()
                .rules()
                .netname_net(&wire.net)
                .unwrap();

            Self::add_path(
                &mut board,
                (0.0, 0.0).into(),
                0.0,
                (0.0, 0.0).into(),
                0.0,
                &wire.path.coords,
                wire.path.width,
                layer,
                net,
                None,
            );
        }

        // The clones here are bad, we'll have something better later on.

        let layername_to_layer = &board.layout().drawing().rules().layer_layername.clone();

        for (layer, layername) in layername_to_layer.iter() {
            board
                .layout_mut()
                .rules_mut()
                .bename_layer(*layer, layername.to_string());
        }

        let netname_to_net = &board.layout().drawing().rules().net_netname.clone();

        for (net, netname) in netname_to_net.iter() {
            board
                .layout_mut()
                .rules_mut()
                .bename_net(*net, netname.to_string());
        }

        board
    }

    fn layer(
        board: &Board<SpecctraMesadata>,
        layers: &Vec<Layer>,
        layername: &str,
        front: bool,
    ) -> usize {
        let image_layer = board
            .layout()
            .drawing()
            .rules()
            .layername_layer(layername)
            .unwrap();

        if front {
            image_layer
        } else {
            layers.len() - image_layer - 1
        }
    }

    fn add_circle(
        board: &mut Board<SpecctraMesadata>,
        place_pos: Point,
        place_rot: f64,
        pin_pos: Point,
        pin_rot: f64,
        r: f64,
        layer: usize,
        net: usize,
        maybe_pin: Option<String>,
    ) {
        let circle = Circle {
            pos: Self::pos(place_pos, place_rot, pin_pos, pin_rot, 0.0, 0.0),
            r,
        };

        board.add_fixed_dot_infringably(
            FixedDotWeight {
                circle,
                layer,
                maybe_net: Some(net),
            },
            maybe_pin.clone(),
        );
    }

    fn add_rect(
        board: &mut Board<SpecctraMesadata>,
        place_pos: Point,
        place_rot: f64,
        pin_pos: Point,
        pin_rot: f64,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        layer: usize,
        net: usize,
        maybe_pin: Option<String>,
    ) {
        let poly = board.add_poly(
            SolidPolyWeight {
                layer,
                maybe_net: Some(net),
            }
            .into(),
            maybe_pin.clone(),
        );

        // Corners.
        let dot_1_1 = board.add_poly_fixed_dot_infringably(
            FixedDotWeight {
                circle: Circle {
                    pos: Self::pos(place_pos, place_rot, pin_pos, pin_rot, x1, y1),
                    r: 0.5,
                },
                layer,
                maybe_net: Some(net),
            },
            poly,
        );
        let dot_2_1 = board.add_poly_fixed_dot_infringably(
            FixedDotWeight {
                circle: Circle {
                    pos: Self::pos(place_pos, place_rot, pin_pos, pin_rot, x2, y1),
                    r: 0.5,
                },
                layer,
                maybe_net: Some(net),
            },
            poly,
        );
        let dot_2_2 = board.add_poly_fixed_dot_infringably(
            FixedDotWeight {
                circle: Circle {
                    pos: Self::pos(place_pos, place_rot, pin_pos, pin_rot, x2, y2),
                    r: 0.5,
                },
                layer,
                maybe_net: Some(net),
            },
            poly,
        );
        let dot_1_2 = board.add_poly_fixed_dot_infringably(
            FixedDotWeight {
                circle: Circle {
                    pos: Self::pos(place_pos, place_rot, pin_pos, pin_rot, x1, y2),
                    r: 0.5,
                },
                layer,
                maybe_net: Some(net),
            },
            poly,
        );
        // Sides.
        board.add_poly_fixed_seg_infringably(
            dot_1_1,
            dot_2_1,
            FixedSegWeight {
                width: 1.0,
                layer,
                maybe_net: Some(net),
            },
            poly,
        );
        board.add_poly_fixed_seg_infringably(
            dot_2_1,
            dot_2_2,
            FixedSegWeight {
                width: 1.0,
                layer,
                maybe_net: Some(net),
            },
            poly,
        );
        board.add_poly_fixed_seg_infringably(
            dot_2_2,
            dot_1_2,
            FixedSegWeight {
                width: 1.0,
                layer,
                maybe_net: Some(net),
            },
            poly,
        );
        board.add_poly_fixed_seg_infringably(
            dot_1_2,
            dot_1_1,
            FixedSegWeight {
                width: 1.0,
                layer,
                maybe_net: Some(net),
            },
            poly,
        );
    }

    fn add_path(
        board: &mut Board<SpecctraMesadata>,
        place_pos: Point,
        place_rot: f64,
        pin_pos: Point,
        pin_rot: f64,
        coords: &Vec<structure::Point>,
        width: f64,
        layer: usize,
        net: usize,
        maybe_pin: Option<String>,
    ) {
        // add the first coordinate in the wire path as a dot and save its index
        let mut prev_pos = Self::pos(
            place_pos,
            place_rot,
            pin_pos,
            pin_rot,
            coords[0].x,
            coords[0].y,
        );
        let mut prev_index = board.add_fixed_dot_infringably(
            FixedDotWeight {
                circle: Circle {
                    pos: prev_pos,
                    r: width / 2.0,
                },
                layer,
                maybe_net: Some(net),
            },
            maybe_pin.clone(),
        );

        // iterate through path coords starting from the second
        for coord in coords.iter().skip(1) {
            let pos = Self::pos(place_pos, place_rot, pin_pos, pin_rot, coord.x, coord.y);

            if pos == prev_pos {
                continue;
            }

            let index = board.add_fixed_dot_infringably(
                FixedDotWeight {
                    circle: Circle {
                        pos,
                        r: width / 2.0,
                    },
                    layer,
                    maybe_net: Some(net),
                },
                maybe_pin.clone(),
            );

            // add a seg between the current and previous coords
            let _ = board.add_fixed_seg_infringably(
                prev_index,
                index,
                FixedSegWeight {
                    width,
                    layer,
                    maybe_net: Some(net),
                },
                maybe_pin.clone(),
            );

            prev_index = index;
            prev_pos = pos;
        }
    }

    fn add_polygon(
        board: &mut Board<SpecctraMesadata>,
        place_pos: Point,
        place_rot: f64,
        pin_pos: Point,
        pin_rot: f64,
        coords: &Vec<structure::Point>,
        width: f64,
        layer: usize,
        net: usize,
        maybe_pin: Option<String>,
    ) {
        let poly = board.add_poly(
            SolidPolyWeight {
                layer,
                maybe_net: Some(net),
            }
            .into(),
            maybe_pin.clone(),
        );

        // add the first coordinate in the wire path as a dot and save its index
        let mut prev_index = board.add_poly_fixed_dot_infringably(
            FixedDotWeight {
                circle: Circle {
                    pos: Self::pos(
                        place_pos,
                        place_rot,
                        pin_pos,
                        pin_rot,
                        coords[0].x,
                        coords[0].y,
                    ),
                    r: width / 2.0,
                },
                layer,
                maybe_net: Some(net),
            },
            // TODO: This manual retagging shouldn't be necessary, `.into()` should suffice.
            //GenericIndex::new(poly.petgraph_index()).into(),
            poly,
        );

        // iterate through path coords starting from the second
        for coord in coords.iter().skip(1) {
            let index = board.add_poly_fixed_dot_infringably(
                FixedDotWeight {
                    circle: Circle {
                        pos: Self::pos(place_pos, place_rot, pin_pos, pin_rot, coord.x, coord.y)
                            .into(),
                        r: width / 2.0,
                    },
                    layer,
                    maybe_net: Some(net),
                },
                // TODO: This manual retagging shouldn't be necessary, `.into()` should suffice.
                poly,
            );

            // add a seg between the current and previous coords
            let _ = board.add_poly_fixed_seg_infringably(
                prev_index,
                index,
                FixedSegWeight {
                    width,
                    layer,
                    maybe_net: Some(net),
                },
                // TODO: This manual retagging shouldn't be necessary, `.into()` should suffice.
                poly,
            );

            prev_index = index;
        }
    }

    fn pos(
        place_pos: Point,
        place_rot: f64,
        pin_pos: Point,
        pin_rot: f64,
        x: f64,
        y: f64,
    ) -> Point {
        let pos = (point! {x: x, y: y} + pin_pos).rotate_around_point(pin_rot, pin_pos);
        (pos + place_pos).rotate_around_point(place_rot, place_pos)
    }
}
