// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use specctra::{
    math::PointWithRotation,
    structure::{DsnFile, Shape},
};

use crate::{board::Board, layout::Joint, math::Vector2};

impl Board {
    pub fn from_specctra(dsn: DsnFile) -> Self {
        let mut board = Board::new(
            dsn.pcb
                .structure
                .boundary
                .coords()
                .into_owned()
                .into_iter()
                .map(|p| [p.x as i64, p.y as i64])
                .collect(),
        );

        // add pins from components
        for component in &dsn.pcb.placement.components {
            let image = dsn
                .pcb
                .library
                .images
                .iter()
                .find(|image| image.name == component.name)
                .unwrap();

            for place in &component.places {
                /*let place_side_is_front = place.side == "front";
                let get_layer = |board: &Board, name: &str| {
                    Self::layer(board, &dsn.pcb.structure.layers, name, place_side_is_front)
                };*/

                for pin in &image.pins {
                    let padstack = dsn.pcb.library.find_padstack_by_name(&pin.name).unwrap();

                    for shape in padstack.shapes.iter() {
                        match shape {
                            Shape::Circle(circle) => Self::place_circle(
                                &mut board,
                                place.point_with_rotation(),
                                pin.point_with_rotation(),
                                (circle.diameter / 2.0) as u64,
                                false,
                            ),
                            _ => (),
                        }
                    }
                }
            }
        }

        board
    }

    pub fn place_circle(
        board: &mut Board,
        place: PointWithRotation,
        pin: PointWithRotation,
        radius: u64,
        flip: bool,
    ) {
        let pos = Self::pos(place, pin, 0.0, 0.0, flip);

        board.add_joint(Joint {
            position: [pos.x, pos.y],
            layer: 0,
            radius,
        });
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
