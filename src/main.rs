extern crate sdl2;

#[macro_use] mod graph;
mod layout;
mod rules;
mod mesh;
mod stretch;
mod primitive;
mod shape;
mod weight;
mod math;

use std::time::Duration;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::gfx::primitives::DrawRenderer;

use crate::layout::Layout;
use crate::graph::DotWeight;
use crate::math::Circle;

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("rust-sdl2 demo", 800, 600)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut i = 0;
    let mut layout = Layout::new();
    
    /*let index = layout.add_dot(DotWeight {net: 0, circle: Circle {pos: (150.5, 80.5).into(), r: 8.0}});
    //layout.route_seg(index, Point {x: 400.5, y: 350.5}, 6.0);

    let index2 = layout.add_dot(DotWeight {net: 0, circle: Circle {pos: (180.5, 150.5).into(), r: 8.0}});
    let barrier1 = layout.add_dot(DotWeight {net: 0, circle: Circle {pos: (90.5, 150.5).into(), r: 8.0}});
    layout.add_seg(index2, barrier1, 16.0);

    let index3 = layout.add_dot(DotWeight {net: 0, circle: Circle {pos: (130.5, 250.5).into(), r: 8.0}});
    let barrier2 = layout.add_dot(DotWeight {net: 0, circle: Circle {pos: (190.5, 250.5).into(), r: 8.0}});
    layout.add_seg(index3, barrier2, 16.0);

    let index4 = layout.route_around_dot(index, index2, true, 5.0);
    let index5 = layout.route_around_dot(index4, index3, false, 5.0);

    let index6 = layout.add_dot(DotWeight {net: 0, circle: Circle {pos: (140.5, 300.5).into(), r: 8.0}});
    let index7 = layout.route_to(index5, index6, 5.0);*/

    let dot1 = layout.add_dot(DotWeight {net: 0, circle: Circle {pos: (100.5, 150.5).into(), r: 8.0}});
    let dot2 = layout.add_dot(DotWeight {net: 0, circle: Circle {pos: (130.5, 150.5).into(), r: 8.0}});
    let dot3 = layout.add_dot(DotWeight {net: 0, circle: Circle {pos: (160.5, 150.5).into(), r: 8.0}});

    let obstacle_dot1 = layout.add_dot(DotWeight {net: 0, circle: Circle {pos: (220.5, 250.5).into(), r: 8.0}});
    let obstacle_dot2 = layout.add_dot(DotWeight {net: 0, circle: Circle {pos: (70.5, 250.5).into(), r: 8.0}});
    layout.add_seg(obstacle_dot1, obstacle_dot2, 16.0);

    let dot4 = layout.add_dot(DotWeight {net: 0, circle: Circle {pos: (180.5, 380.5).into(), r: 8.0}});
    let dot5 = layout.add_dot(DotWeight {net: 0, circle: Circle {pos: (220.5, 380.5).into(), r: 8.0}});
    let dot6 = layout.add_dot(DotWeight {net: 0, circle: Circle {pos: (290.5, 380.5).into(), r: 8.0}});

    let head = layout.route_start(dot3);
    let head = layout.route_around_dot(head, obstacle_dot1, true, 5.0);
    let dot3_1 = head.dot;
    let bend3_1 = head.bend.unwrap();
    layout.route_stop(head, dot4, 5.0);

    let head = layout.route_start(dot2);
    let head = layout.route_around_dot(head, dot3, true, 5.0);
    let dot2_1 = head.dot;
    let bend2_1 = head.bend.unwrap();
    let head = layout.route_around_bend(head, bend3_1, true, 5.0);
    let dot2_2 = head.dot;
    let bend2_2 = head.bend.unwrap();
    layout.route_stop(head, dot5, 5.0);

    let head = layout.route_start(dot1);
    let head = layout.route_around_bend(head, bend2_1, true, 5.0);
    let head = layout.route_around_bend(head, bend2_2, true, 5.0);
    layout.route_stop(head, dot6, 5.0);

    'running: loop {
        i = (i + 1) % 255;

        canvas.set_draw_color(Color::RGB(0, 10, 35));
        canvas.clear();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                }
                _ => {}
            }
        }

        for shape in layout.shapes() {
            if let Some(center) = shape.center {
                let circle = shape.circle().unwrap();
                let delta1 = shape.from - circle.pos;
                let delta2 = shape.to - circle.pos;

                let mut angle1 = delta1.y().atan2(delta1.x());
                let mut angle2 = delta2.y().atan2(delta2.x());

                for d in -3..3 {
                    let _ = canvas.arc(
                        //around_circle.pos.x() as i16,
                        //around_circle.pos.y() as i16,
                        circle.pos.x() as i16,
                        circle.pos.y() as i16,
                        //(shape.around_weight.unwrap().circle.r + 10.0 + (d as f64)) as i16,
                        (circle.r + (d as f64)) as i16,
                        angle1.to_degrees() as i16,
                        angle2.to_degrees() as i16,
                        Color::RGB(200, 52, 52));
                }
            } else if shape.from != shape.to {
                let _ = canvas.thick_line(shape.from.x() as i16,
                                          shape.from.y() as i16,
                                          shape.to.x() as i16,
                                          shape.to.y() as i16,
                                          shape.width as u8,
                                          Color::RGB(200, 52, 52));
            } else {
                let _ = canvas.filled_circle(shape.from.x() as i16,
                                             shape.from.y() as i16,
                                             (shape.width / 2.0) as i16,
                                             Color::RGB(200, 52, 52));
            }
        }

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
