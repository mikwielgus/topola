// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeMap;

use bimap::BiBTreeMap;
use specctra::{
    math::PointWithRotation,
    structure::{DsnFile, Layer, Point, Shape},
};

use crate::{
    board::Board,
    compounds::{ComponentId, NetId, PinId},
    math::Vector2,
    primitives::{JointSpec, Polygon, Segment, SegmentSpec},
};

impl Board {
    pub fn from_specctra(dsn: DsnFile) -> Self {
        let layer_names = BiBTreeMap::from_iter(
            dsn.pcb
                .structure
                .layers
                .iter()
                .enumerate()
                .map(|(index, layer)| (index, layer.name.clone())),
        );

        // assign IDs to all nets named in pcb.network
        let net_names = {
            let mut tmp: Vec<_> = dsn
                .pcb
                .network
                .classes
                .iter()
                .flat_map(|class| &class.nets)
                .chain(dsn.pcb.network.nets.iter().map(|net| &net.name))
                .collect();
            // deduplicate net names
            tmp.sort_unstable();
            tmp.dedup();

            BiBTreeMap::from_iter(
                tmp.into_iter()
                    .cloned()
                    .enumerate()
                    .map(|(i, v)| (NetId::new(i), v)),
            )
        };

        let mut board = Board::with_names(
            dsn.pcb
                .structure
                .boundary
                .coords()
                .into_owned()
                .into_iter()
                .skip(1)
                .rev()
                .map(|p| Vector2::new(p.x as i64, p.y as i64))
                .collect(),
            dsn.pcb.structure.layers.len(),
            layer_names,
            net_names,
        );

        // Mapping of pin -> net prepared for adding pins.
        let pin_nets: BTreeMap<String, NetId> = dsn
            .pcb
            .network
            .nets
            .iter()
            .filter_map(|net_pin_assignments| {
                // Resolve the id so we don't work with strings.
                let net = board.net_id(&net_pin_assignments.name).unwrap();

                net_pin_assignments.pins.as_ref().map(|pins| {
                    // Take the list of pins
                    // and for each pin output (pin name, net id).
                    pins.names.iter().map(move |pinname| (pinname.clone(), net))
                })
            })
            // Flatten the nested iters into a single stream of tuples.
            .flatten()
            .collect();

        // Add pins from components.
        for component in &dsn.pcb.placement.components {
            let image = dsn
                .pcb
                .library
                .images
                .iter()
                .find(|image| image.name == component.name)
                .unwrap();

            for place in &component.places {
                let component_id = board.ensure_named_component(place.name.clone());

                let place_side_is_front = place.side == "front";
                let get_layer = |board: &Board, name: &str| {
                    Self::layer(board, &dsn.pcb.structure.layers, name, place_side_is_front)
                };

                for pin in &image.pins {
                    let pin_name = format!("{}-{}", place.name, pin.id);

                    let Some(net) = pin_nets.get(&pin_name).copied() else {
                        continue;
                    };

                    let pin_id = board.ensure_named_pin(pin_name.clone());
                    let padstack = dsn.pcb.library.find_padstack_by_name(&pin.name).unwrap();

                    for shape in padstack.shapes.iter() {
                        match shape {
                            Shape::Circle(circle) => {
                                let layer = get_layer(&board, &circle.layer);
                                Self::place_circle(
                                    &mut board,
                                    place.point_with_rotation(),
                                    pin.point_with_rotation(),
                                    (circle.diameter / 2.0) as u64,
                                    layer,
                                    net,
                                    Some(component_id),
                                    Some(pin_id),
                                    !place_side_is_front,
                                )
                            }
                            Shape::Rect(rect) => {
                                let layer = get_layer(&board, &rect.layer);
                                Self::place_rect(
                                    &mut board,
                                    place.point_with_rotation(),
                                    pin.point_with_rotation(),
                                    rect.x1,
                                    rect.y1,
                                    rect.x2,
                                    rect.y2,
                                    layer,
                                    net,
                                    Some(component_id),
                                    Some(pin_id),
                                    !place_side_is_front,
                                )
                            }
                            Shape::Path(path) => {
                                let layer = get_layer(&board, &path.layer);
                                Self::place_path(
                                    &mut board,
                                    place.point_with_rotation(),
                                    pin.point_with_rotation(),
                                    &path.coords,
                                    path.width,
                                    layer,
                                    net,
                                    Some(component_id),
                                    Some(pin_id),
                                    !place_side_is_front,
                                )
                            }
                            Shape::Polygon(polygon) => {
                                let layer = get_layer(&board, &polygon.layer);
                                Self::place_polygon(
                                    &mut board,
                                    place.point_with_rotation(),
                                    pin.point_with_rotation(),
                                    &polygon.coords,
                                    polygon.width,
                                    layer,
                                    net,
                                    Some(component_id),
                                    Some(pin_id),
                                    !place_side_is_front,
                                )
                            }
                        }
                    }
                }
            }
        }

        for via in &dsn.pcb.wiring.vias {
            let net = board.net_id(&via.net).unwrap();
            let padstack = dsn.pcb.library.find_padstack_by_name(&via.name).unwrap();

            let get_layer = |board: &Board, name: &str| {
                Self::layer(board, &dsn.pcb.structure.layers, name, true)
            };

            for shape in &padstack.shapes {
                match shape {
                    Shape::Circle(circle) => {
                        let layer = get_layer(&board, &circle.layer);
                        Self::place_circle(
                            &mut board,
                            PointWithRotation::from_xy(via.x, via.y),
                            PointWithRotation::default(),
                            (circle.diameter / 2.0) as u64,
                            layer,
                            net,
                            None,
                            None,
                            false,
                        )
                    }
                    Shape::Rect(rect) => {
                        let layer = get_layer(&board, &rect.layer);
                        Self::place_rect(
                            &mut board,
                            PointWithRotation::from_xy(via.x, via.y),
                            PointWithRotation::default(),
                            rect.x1,
                            rect.y1,
                            rect.x2,
                            rect.y2,
                            layer,
                            net,
                            None,
                            None,
                            false,
                        )
                    }
                    Shape::Path(path) => {
                        let layer = get_layer(&board, &path.layer);
                        Self::place_path(
                            &mut board,
                            PointWithRotation::from_xy(via.x, via.y),
                            PointWithRotation::default(),
                            &path.coords,
                            path.width,
                            layer,
                            net,
                            None,
                            None,
                            false,
                        )
                    }
                    Shape::Polygon(polygon) => {
                        let layer = get_layer(&board, &polygon.layer);
                        Self::place_polygon(
                            &mut board,
                            PointWithRotation::from_xy(via.x, via.y),
                            PointWithRotation::default(),
                            &polygon.coords,
                            polygon.width,
                            layer,
                            net,
                            None,
                            None,
                            false,
                        )
                    }
                };
            }
        }

        for wire in dsn.pcb.wiring.wires.iter() {
            let layer = board.layer_id(&wire.path.layer).unwrap();
            let net = board.net_id(&wire.net).unwrap();

            Self::place_path(
                &mut board,
                PointWithRotation::default(),
                PointWithRotation::default(),
                &wire.path.coords,
                wire.path.width,
                layer,
                net,
                None,
                None,
                false,
            );
        }

        board
    }

    fn place_circle(
        board: &mut Board,
        place: PointWithRotation,
        pin_pos: PointWithRotation,
        radius: u64,
        layer: usize,
        net: NetId,
        component: Option<ComponentId>,
        pin: Option<PinId>,
        flip: bool,
    ) {
        board.add_joint(JointSpec {
            position: Self::pos(place, pin_pos, 0.0, 0.0, flip),
            layer,
            net,
            component,
            pin,
            radius,
        });
    }

    fn place_rect(
        board: &mut Board,
        place: PointWithRotation,
        pin_pos: PointWithRotation,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        layer: usize,
        net: NetId,
        component: Option<ComponentId>,
        pin: Option<PinId>,
        flip: bool,
    ) {
        board.add_polygon(Polygon {
            vertices: vec![
                Self::pos(place, pin_pos, x1, y1, flip),
                Self::pos(place, pin_pos, x2, y1, flip),
                Self::pos(place, pin_pos, x2, y2, flip),
                Self::pos(place, pin_pos, x1, y2, flip),
            ],
            layer,
            net,
            component,
            pin,
        });
    }

    fn place_path(
        board: &mut Board,
        place: PointWithRotation,
        pin_pos: PointWithRotation,
        coords: &[Point],
        width: f64,
        layer: usize,
        net: NetId,
        component: Option<ComponentId>,
        pin: Option<PinId>,
        flip: bool,
    ) {
        // Add the first coordinate in the wire path as a dot and save its index.
        let mut prev_pos: Vector2<i64> = Self::pos(place, pin_pos, coords[0].x, coords[0].y, flip);
        let mut prev_joint = board.add_joint(JointSpec {
            position: prev_pos,
            layer,
            radius: (width / 2.0) as u64,
            net,
            component,
            pin,
        });

        // Iterate through path coords starting from the second.
        for coord in coords.iter().skip(1) {
            let pos = Self::pos(place, pin_pos, coord.x, coord.y, flip);

            if pos == prev_pos {
                continue;
            }

            let joint = board.add_joint(JointSpec {
                position: pos,
                layer,
                radius: (width / 2.0) as u64,
                net,
                component,
                pin,
            });

            // Add a seg between the current and previous coords.
            let _ = board.add_segment_raw(Segment {
                spec: SegmentSpec {
                    endjoints: [prev_joint, joint],
                    half_width: (width / 2.0) as u64,
                    component,
                    pin,
                },
                endpoints: [prev_pos, pos],
                layer,
                net,
            });

            prev_pos = pos;
            prev_joint = joint;
        }
    }

    fn place_polygon(
        board: &mut Board,
        place: PointWithRotation,
        pin_pos: PointWithRotation,
        coords: &[Point],
        _width: f64,
        layer: usize,
        net: NetId,
        component: Option<ComponentId>,
        pin: Option<PinId>,
        flip: bool,
    ) {
        let vertices: Vec<Vector2<i64>> = coords
            .iter()
            .map(|coord| Self::pos(place, pin_pos, coord.x, coord.y, flip))
            .collect();
        board.add_polygon(Polygon {
            vertices,
            layer,
            net,
            component,
            pin,
        });
    }

    fn layer(board: &Board, layers: &[Layer], name: &str, front: bool) -> usize {
        let image_layer = board.layer_id(name).unwrap();

        if front {
            image_layer
        } else {
            layers.len() - image_layer - 1
        }
    }

    fn pos(
        place: PointWithRotation,
        pin: PointWithRotation,
        x: f64,
        y: f64,
        flip: bool,
    ) -> Vector2<i64> {
        let pos = (Vector2::new(x, y) + Vector2::new(pin.pos.x(), pin.pos.y()))
            .rotate_around_point_degrees(pin.rot, Vector2::new(pin.pos.x(), pin.pos.y()));
        let pos = (Vector2::new(place.pos.x(), place.pos.y())
            + flip.then_some(Vector2::new(-pos.x, pos.y)).unwrap_or(pos))
        .rotate_around_point_degrees(place.rot, Vector2::new(place.pos.x(), place.pos.y()));

        Vector2::new(pos.x as i64, pos.y as i64)
    }
}
