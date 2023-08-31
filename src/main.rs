extern crate sdl2;

macro_rules! dbg_dot {
    ($graph:expr) => {
        use petgraph::dot::Dot;
        println!("{:?}", Dot::new(&$graph));
    };
}

#[macro_use]
mod graph;
mod astar;
mod bow;
mod draw;
mod guide;
mod layout;
mod math;
mod mesh;
mod primitive;
mod route;
mod router;
mod rules;
mod segbend;
mod shape;

use graph::{Tag, TaggedIndex};
use sdl2::event::Event;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::EventPump;
use shape::Shape;
use std::panic;
use std::time::Duration;

use crate::graph::DotWeight;
use crate::math::Circle;
use crate::router::Router;

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("rust-sdl2 demo", 800, 600)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let _i = 0;
    let mut router = Router::new();

    let dot1_1 = router
        .layout
        .add_dot(DotWeight {
            net: 1,
            circle: Circle {
                pos: (100.5, 400.5).into(),
                r: 8.0,
            },
        })
        .unwrap();
    let _dot2_1 = router
        .layout
        .add_dot(DotWeight {
            net: 2,
            circle: Circle {
                pos: (130.5, 430.5).into(),
                r: 8.0,
            },
        })
        .unwrap();
    let _dot3_1 = router
        .layout
        .add_dot(DotWeight {
            net: 3,
            circle: Circle {
                pos: (160.5, 460.5).into(),
                r: 8.0,
            },
        })
        .unwrap();
    let _dot4_1 = router
        .layout
        .add_dot(DotWeight {
            net: 4,
            circle: Circle {
                pos: (190.5, 490.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let dot1_2 = router
        .layout
        .add_dot(DotWeight {
            net: 1,
            circle: Circle {
                pos: (700.5, 400.5).into(),
                r: 8.0,
            },
        })
        .unwrap();
    let _dot2_2 = router
        .layout
        .add_dot(DotWeight {
            net: 2,
            circle: Circle {
                pos: (670.5, 430.5).into(),
                r: 8.0,
            },
        })
        .unwrap();
    let _dot3_2 = router
        .layout
        .add_dot(DotWeight {
            net: 3,
            circle: Circle {
                pos: (640.5, 460.5).into(),
                r: 8.0,
            },
        })
        .unwrap();
    let _dot4_2 = router
        .layout
        .add_dot(DotWeight {
            net: 4,
            circle: Circle {
                pos: (610.5, 490.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let _dot5 = router
        .layout
        .add_dot(DotWeight {
            net: 5,
            circle: Circle {
                pos: (150.5, 100.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let _dot6 = router
        .layout
        .add_dot(DotWeight {
            net: 6,
            circle: Circle {
                pos: (190.5, 200.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let _dot7 = router
        .layout
        .add_dot(DotWeight {
            net: 5,
            circle: Circle {
                pos: (230.5, 70.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let barrier1_dot1 = router
        .layout
        .add_dot(DotWeight {
            net: 10,
            circle: Circle {
                pos: (250.5, 250.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let _barrier2_dot1 = router
        .layout
        .add_dot(DotWeight {
            net: 20,
            circle: Circle {
                pos: (420.5, 200.5).into(),
                r: 8.0,
            },
        })
        .unwrap();
    let _barrier2_dot2 = router
        .layout
        .add_dot(DotWeight {
            net: 20,
            circle: Circle {
                pos: (480.5, 700.5).into(),
                r: 8.0,
            },
        })
        .unwrap();
    /*let _ = router.layout.add_seg(
        barrier2_dot1,
        barrier2_dot2,
        SegWeight {
            net: 20,
            width: 16.0,
        },
    );*/

    /*let head = router.draw_start(dot5);
    let head = router.draw_around_dot(head, dot6, false, 5.0).unwrap();
    let _ = router.draw_finish(head, dot7, 5.0);*/

    router.enroute(dot1_1, dot1_2);

    render_times(&mut event_pump, &mut canvas, &mut router, None, -1);
    render_times(
        &mut event_pump,
        &mut canvas,
        &mut router,
        Some(barrier1_dot1.tag()),
        -1,
    );
    render_times(&mut event_pump, &mut canvas, &mut router, None, -1);
}

fn render_times(
    event_pump: &mut EventPump,
    canvas: &mut Canvas<Window>,
    router: &mut Router,
    follower: Option<TaggedIndex>,
    times: i64,
) {
    let mut i = 0;

    'running: loop {
        canvas.set_draw_color(Color::RGB(0, 10, 35));
        canvas.clear();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        if let Some(follower) = follower {
            let state = event_pump.mouse_state();

            /*let _ = router.move_dot(
                *follower.as_dot().unwrap(),
                (state.x() as f64, state.y() as f64).into(),
            );*/
        }

        let result = panic::catch_unwind(|| {
            for shape in router.layout.shapes() {
                match shape {
                    Shape::Dot(dot) => {
                        let _ = canvas.filled_circle(
                            dot.c.pos.x() as i16,
                            dot.c.pos.y() as i16,
                            dot.c.r as i16,
                            Color::RGB(200, 52, 52),
                        );
                    }
                    Shape::Seg(seg) => {
                        let _ = canvas.thick_line(
                            seg.from.x() as i16,
                            seg.from.y() as i16,
                            seg.to.x() as i16,
                            seg.to.y() as i16,
                            seg.width as u8,
                            Color::RGB(200, 52, 52),
                        );
                    }
                    Shape::Bend(bend) => {
                        let delta1 = bend.from - bend.c.pos;
                        let delta2 = bend.to - bend.c.pos;

                        let angle1 = delta1.y().atan2(delta1.x());
                        let angle2 = delta2.y().atan2(delta2.x());

                        for d in -2..3 {
                            let _ = canvas.arc(
                                //around_circle.pos.x() as i16,
                                //around_circle.pos.y() as i16,
                                bend.c.pos.x() as i16,
                                bend.c.pos.y() as i16,
                                //(shape.around_weight.unwrap().circle.r + 10.0 + (d as f64)) as i16,
                                (bend.circle().r + (d as f64)) as i16,
                                angle1.to_degrees() as i16,
                                angle2.to_degrees() as i16,
                                Color::RGB(200, 52, 52),
                            );
                        }
                    }
                }
                /*let envelope = shape.envelope();
                let _ = canvas.rectangle(
                    envelope.lower()[0] as i16,
                    envelope.lower()[1] as i16,
                    envelope.upper()[0] as i16,
                    envelope.upper()[1] as i16,
                    Color::RGB(100, 100, 100),
                );*/
            }

            /*for edge in router.routeedges() {
                let _ = canvas.line(
                    edge.0.x() as i16,
                    edge.0.y() as i16,
                    edge.1.x() as i16,
                    edge.1.y() as i16,
                    Color::RGB(250, 250, 250),
                );
            }*/
        });

        if let Err(err) = result {
            dbg!(err);
        }

        canvas.present();

        i += 1;
        if times != -1 && i >= times {
            return;
        }

        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
