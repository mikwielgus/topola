#![feature(try_blocks)]

mod astar;
mod draw;
mod graph;
#[macro_use]
mod layout;
mod math;
mod mesh;
mod router;
mod tracer;
mod triangulation;
mod wraparoundable;

#[cfg_attr(feature = "sdl2_demo", path = "app/sdl2_demo/app.rs")]
mod app;

#[cfg(feature = "sdl2_demo")]
pub fn run() -> Result<(), anyhow::Error> {
    app::run()
}

fn main() -> Result<(), anyhow::Error> {
    run()
}
