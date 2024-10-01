//! Module for managing the various Specctra PCB design, including loading the
//! Design DSN file, creating the [`Board`] object from the file, as well as
//! exporting the session file
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
    math::{Circle, PointWithRotation},
    specctra::{
        mesadata::SpecctraMesadata,
        read::{self, ListTokenizer},
        structure::{self, DsnFile, Layer, Pcb, Shape},
        write::ListWriter,
    },
};

#[derive(Error, Debug)]

/// Errors raised by [`SpecctraDesign::load`]
pub enum LoadingError {
    /// I/O file reading error from [`std::io::Error`]
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// File parsing errors containing information about unexpected end of file,
    /// or any other parsing issues with provided DSN file
    #[error(transparent)]
    Parse(#[from] read::ParseErrorContext),
}

/// This struct is responsible for managing the various Specctra components of a PCB design,
/// including parsing the DSN file, handling the resolution, unit of measurement,
/// and organizing the PCB's structure, placement, library, network, and wiring.
/// It provides functionality for reading from a DSN file and writing Specctra's .SES session files.
#[derive(Debug)]
pub struct SpecctraDesign {
    pcb: Pcb,
}

impl SpecctraDesign {
    /// Loads a [`SpecctraDesign`] structure instance from a buffered reader.
    ///
    /// This function reads the Specctra Design data from an input stream.
    /// Later the data is parsed and loaded into a [`SpecctraDesign`] structure,
    /// allowing further operations such as rule validation, routing, or netlist management.
    pub fn load(reader: impl std::io::BufRead) -> Result<SpecctraDesign, LoadingError> {
        let mut list_reader = ListTokenizer::new(reader);
        let dsn = list_reader.read_value::<DsnFile>()?;

        Ok(Self { pcb: dsn.pcb })
    }

    /// Function to get name of the DSN file
    ///
    /// This function returns the name of the `Pcb` objects
    pub fn get_name(&self) -> &str {
        &self.pcb.name
    }

    /// Writes the Specctra Session (.ses) file format using the current board layout and mesadata.
    ///
    /// This function generates a Specctra SES session file that represents the board's net routing and
    /// writes it to the provided output stream. The session data includes routed nets, wires,
    /// layers, and other essential information for routing management.
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

    /// Generates a [`Board<SpecctraMesadata>`] from the current PCB data.
    ///
    /// This function takes the internal `Pcb` structure and transforms it into a [`Board`] object,
    /// which is used for layout and routing operations. The board is initialized with [`SpecctraMesadata`],
    /// which includes layer and net mappings, and is populated with components, pins, vias, and wires
    /// from the PCB definition.
    pub fn make_board(&self) -> Board<SpecctraMesadata> {
        let mesadata = SpecctraMesadata::from_pcb(&self.pcb);
        let mut board = Board::new(Layout::new(Drawing::new(
            mesadata,
            self.pcb.structure.layers.len(),
        )));

        // mapping of pin -> net prepared for adding pins
        let pin_nets = self
            .pcb
            .network
            .nets
            .iter()
            // flatten the nested iters into a single stream of tuples
            .flat_map(|net_pin_assignments| {
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
            .collect::<HashMap<String, usize>>();

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

                let place_side_is_front = place.side == "front";
                let get_layer = |board: &Board<SpecctraMesadata>, name: &str| {
                    Self::layer(board, &self.pcb.structure.layers, name, place_side_is_front)
                };

                for pin in &image.pins {
                    let pinname = format!("{}-{}", place.name, pin.id);
                    let net = pin_nets.get(&pinname).unwrap();

                    let padstack = self.pcb.library.find_padstack_by_name(&pin.name).unwrap();

                    for shape in padstack.shapes.iter() {
                        match shape {
                            Shape::Circle(circle) => {
                                let layer = get_layer(&board, &circle.layer);
                                Self::add_circle(
                                    &mut board,
                                    place.point_with_rotation(),
                                    pin.point_with_rotation(),
                                    circle.diameter / 2.0,
                                    layer,
                                    *net,
                                    Some(pinname.clone()),
                                )
                            }
                            Shape::Rect(rect) => {
                                let layer = get_layer(&board, &rect.layer);
                                Self::add_rect(
                                    &mut board,
                                    place.point_with_rotation(),
                                    pin.point_with_rotation(),
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
                                let layer = get_layer(&board, &path.layer);
                                Self::add_path(
                                    &mut board,
                                    place.point_with_rotation(),
                                    pin.point_with_rotation(),
                                    &path.coords,
                                    path.width,
                                    layer,
                                    *net,
                                    Some(pinname.clone()),
                                )
                            }
                            Shape::Polygon(polygon) => {
                                let layer = get_layer(&board, &polygon.layer);
                                Self::add_polygon(
                                    &mut board,
                                    place.point_with_rotation(),
                                    pin.point_with_rotation(),
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

            let padstack = self.pcb.library.find_padstack_by_name(&via.name).unwrap();

            let get_layer = |board: &Board<SpecctraMesadata>, name: &str| {
                Self::layer(board, &self.pcb.structure.layers, name, true)
            };

            for shape in &padstack.shapes {
                match shape {
                    Shape::Circle(circle) => {
                        let layer = get_layer(&board, &circle.layer);
                        Self::add_circle(
                            &mut board,
                            PointWithRotation::default(),
                            PointWithRotation::default(),
                            circle.diameter / 2.0,
                            layer,
                            net,
                            None,
                        )
                    }
                    Shape::Rect(rect) => {
                        let layer = get_layer(&board, &rect.layer);
                        Self::add_rect(
                            &mut board,
                            PointWithRotation::default(),
                            PointWithRotation::default(),
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
                        let layer = get_layer(&board, &path.layer);
                        Self::add_path(
                            &mut board,
                            PointWithRotation::default(),
                            PointWithRotation::default(),
                            &path.coords,
                            path.width,
                            layer,
                            net,
                            None,
                        )
                    }
                    Shape::Polygon(polygon) => {
                        let layer = get_layer(&board, &polygon.layer);
                        Self::add_polygon(
                            &mut board,
                            PointWithRotation::default(),
                            PointWithRotation::default(),
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
                PointWithRotation::default(),
                PointWithRotation::default(),
                &wire.path.coords,
                wire.path.width,
                layer,
                net,
                None,
            );
        }

        board
    }

    fn layer(
        board: &Board<SpecctraMesadata>,
        layers: &[Layer],
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
        place: PointWithRotation,
        pin: PointWithRotation,
        r: f64,
        layer: usize,
        net: usize,
        maybe_pin: Option<String>,
    ) {
        let circle = Circle {
            pos: Self::pos(place, pin, 0.0, 0.0),
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
        place: PointWithRotation,
        pin: PointWithRotation,
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
                    pos: Self::pos(place, pin, x1, y1),
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
                    pos: Self::pos(place, pin, x2, y1),
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
                    pos: Self::pos(place, pin, x2, y2),
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
                    pos: Self::pos(place, pin, x1, y2),
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
        place: PointWithRotation,
        pin: PointWithRotation,
        coords: &[structure::Point],
        width: f64,
        layer: usize,
        net: usize,
        maybe_pin: Option<String>,
    ) {
        // add the first coordinate in the wire path as a dot and save its index
        let mut prev_pos = Self::pos(place, pin, coords[0].x, coords[0].y);
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
            let pos = Self::pos(place, pin, coord.x, coord.y);

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
        place: PointWithRotation,
        pin: PointWithRotation,
        coords: &[structure::Point],
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
                    pos: Self::pos(place, pin, coords[0].x, coords[0].y),
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
                        pos: Self::pos(place, pin, coord.x, coord.y),
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

    fn pos(place: PointWithRotation, pin: PointWithRotation, x: f64, y: f64) -> Point {
        let pos = (point! {x: x, y: y} + pin.pos).rotate_around_point(pin.rot, pin.pos);
        (pos + place.pos).rotate_around_point(place.rot, place.pos)
    }
}
