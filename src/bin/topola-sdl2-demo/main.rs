extern crate sdl2;

mod painter;

macro_rules! dbg_dot {
    ($graph:expr) => {
        use petgraph::dot::Dot;
        println!("{:?}", Dot::new(&$graph));
    };
}

use geo::point;
use painter::Painter;
use petgraph::graph::NodeIndex;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use topola::autorouter::selection::Selection;
use topola::autorouter::{Autorouter, AutorouterStatus};
use topola::drawing::dot::FixedDotWeight;
use topola::drawing::graph::{MakePrimitive, PrimitiveIndex};
use topola::drawing::primitive::MakePrimitiveShape;
use topola::drawing::rules::{AccessRules, Conditions};
use topola::drawing::seg::FixedSegWeight;
use topola::drawing::{Infringement, LayoutException};
use topola::geometry::primitive::{AccessPrimitiveShape, PrimitiveShape};
use topola::geometry::shape::AccessShape;
use topola::layout::zone::MakePolyShape;
use topola::layout::Layout;
use topola::router::draw::DrawException;
use topola::router::navmesh::Navmesh;
use topola::router::tracer::Tracer;
use topola::specctra::design::SpecctraDesign;
use topola::specctra::mesadata::SpecctraMesadata;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::{GLProfile, Window};
use sdl2::EventPump;

use pathfinder_canvas::{Canvas, CanvasFontContext, ColorU};
use pathfinder_geometry::vector::{vec2f, vec2i, Vector2F};
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::concurrent::rayon::RayonExecutor;
use pathfinder_renderer::concurrent::scene_proxy::SceneProxy;
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererMode, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_renderer::options::BuildOptions;
use pathfinder_resources::embedded::EmbeddedResourceLoader;

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

use topola::math::Circle;
use topola::router::Router;

struct SimpleRules {
    net_clearances: HashMap<(usize, usize), f64>,
}

impl AccessRules for SimpleRules {
    fn clearance(&self, conditions1: &Conditions, conditions2: &Conditions) -> f64 {
        if let (Some(net1), Some(net2)) = (conditions1.maybe_net, conditions2.maybe_net) {
            *self.net_clearances.get(&(net1, net2)).unwrap_or(&10.0)
        } else {
            10.0
        }
    }

    fn largest_clearance(&self, maybe_net: Option<usize>) -> f64 {
        let mut highest_clearance = 0.0;

        if let Some(net) = maybe_net {
            for ((net1, net2), clearance) in self.net_clearances.iter() {
                if *net1 == net || *net2 == net {
                    highest_clearance = *clearance;
                }
            }
        }

        highest_clearance
    }
}

// Clunky enum to work around borrow checker.
enum RouterOrLayout<'a, R: AccessRules> {
    Router(&'a mut Router<'a, R>),
    Layout(&'a Layout<R>),
}

fn main() -> Result<(), anyhow::Error> {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(GLProfile::Core);
    gl_attr.set_context_version(4, 0);

    let window = video_subsystem
        .window("Topola demo", 800, 600)
        .opengl()
        .position_centered()
        .build()
        .unwrap();

    let _context = window.gl_create_context().unwrap();
    gl::load_with(|name| video_subsystem.gl_get_proc_address(name) as *const _);

    // doing this later (after pathfinder assumes control of the context) would be a bad idea
    // but for the early clear it's simpler than passing a blank canvas to pathfinder
    unsafe {
        gl::ClearColor(0.0, 0.0, 0.0, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);
    }
    window.gl_swap_window();

    // XXX: not sure if this automatically fallbacks if we get a GL3 context
    // or if we have to detect it and/or retry
    let device = GLDevice::new(GLVersion::GL4, 0);

    let mode = RendererMode::default_for_device(&device);
    let options = RendererOptions {
        dest: DestFramebuffer::full_window(vec2i(800, 600)),
        background_color: Some(ColorU::black().to_f32()),
        //show_debug_ui: true,
        ..RendererOptions::default()
    };
    let resource_loader = EmbeddedResourceLoader::new();
    let mut renderer = Renderer::new(device, &resource_loader, mode, options);
    let font_context = CanvasFontContext::from_system_source();

    // TODO: make a type like this wrapping the details of pathfinder
    // so we don't pass so many arguments to render_times()
    //let mut canvas = window.into_canvas().build().unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let _i = 0;
    /*let mut router = Router::new(Layout::new(SimpleRules {
        net_clearances: HashMap::from([
            ((1, 2), 8.0),
            ((2, 1), 8.0),
            ((2, 3), 3.0),
            ((3, 2), 3.0),
            ((3, 4), 15.0),
            ((4, 3), 15.0),
        ]),
    }));*/

    let design_file = File::open(
        "tests/single_layer/data/de9_tht_female_to_tht_female/de9_tht_female_to_tht_female.dsn",
    )
    .unwrap();
    let design_bufread = BufReader::new(design_file);
    let design = SpecctraDesign::load(design_bufread)?;
    let board = design.make_board();

    let mut view = View {
        pan: vec2f(-80000.0, -60000.0),
        zoom: 0.005,
    };

    render_times(
        &mut event_pump,
        &window,
        &mut renderer,
        &font_context,
        &mut view,
        RouterOrLayout::Layout(board.layout()),
        None,
        None,
        &[],
        &[],
        &[],
        -1,
    );

    let mut autorouter = Autorouter::new(board).unwrap();
    if let Ok(mut autoroute) = autorouter.autoroute_walk(&Selection::new()) {
        loop {
            let status = match autoroute.step(&mut autorouter) {
                Ok(status) => status,
                Err(err) => break,
            };

            if let AutorouterStatus::Finished = status {
                break;
            }
        }
    }

    // these are both on net 1 in the test file
    /*let _ = router.route_band(
        dot_indices[1],
        dot_indices[2],
        3.0,
    )?;*/

    render_times(
        &mut event_pump,
        &window,
        &mut renderer,
        &font_context,
        &mut view,
        RouterOrLayout::Layout(autorouter.board().layout()),
        None,
        None,
        &[],
        &[],
        &[],
        -1,
    );

    Ok(())
}

struct View {
    pan: Vector2F,
    zoom: f32,
}

fn render_times(
    event_pump: &mut EventPump,
    window: &Window,
    renderer: &mut Renderer<GLDevice>,
    font_context: &CanvasFontContext,
    view: &mut View,
    mut router_or_layout: RouterOrLayout<impl AccessRules>,
    _unused: Option<()>,
    mut maybe_navmesh: Option<Navmesh>,
    path: &[NodeIndex<usize>],
    ghosts: &[PrimitiveShape],
    highlighteds: &[PrimitiveIndex],
    times: i64,
) {
    let mut i = 0;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::MouseWheel { y, .. } => {
                    view.zoom *= f32::powf(1.4, y as f32);
                }
                Event::MouseMotion {
                    xrel,
                    yrel,
                    mousestate,
                    ..
                } => {
                    if mousestate.left() {
                        view.pan += vec2f(xrel as f32, yrel as f32) / view.zoom;
                    }
                }
                _ => {}
            }
        }

        renderer.options_mut().background_color = Some(ColorU::new(0, 10, 35, 255).to_f32());

        let window_size = window.size();
        let mut canvas = Canvas::new(vec2f(window_size.0 as f32, window_size.1 as f32))
            .get_context_2d(font_context.clone());

        let center = vec2f(400.0, 300.0);
        canvas.translate(center);
        canvas.scale(vec2f(view.zoom, view.zoom));
        canvas.translate(-center + view.pan);

        let mut painter = Painter::new(&mut canvas);

        let layout = match router_or_layout {
            RouterOrLayout::Router(ref mut router) => {
                let state = event_pump.mouse_state();

                /*if let Some(band) = maybe_band {
                    router
                        .reroute_band(
                            band,
                            point! {x: state.x() as f64, y: state.y() as f64},
                            3.0,
                        )
                        .ok();
                    maybe_navmesh = None;
                }*/

                router.layout()
            }
            RouterOrLayout::Layout(ref layout) => layout.clone(),
        };

        for node in layout.drawing().layer_primitive_nodes(1) {
            let color = if highlighteds.contains(&node) {
                ColorU::new(100, 100, 255, 255)
            } else {
                ColorU::new(52, 52, 200, 255)
            };

            let shape = node.primitive(layout.drawing()).shape();
            painter.paint_primitive(&shape, color, view.zoom);
        }

        for zone in layout.layer_zone_nodes(1) {
            painter.paint_polygon(
                &layout.zone(zone).shape().polygon,
                ColorU::new(52, 52, 200, 255),
                view.zoom,
            );
        }

        for node in layout.drawing().layer_primitive_nodes(0) {
            let color = if highlighteds.contains(&node) {
                ColorU::new(255, 100, 100, 255)
            } else {
                ColorU::new(200, 52, 52, 255)
            };

            let shape = node.primitive(layout.drawing()).shape();
            painter.paint_primitive(&shape, color, view.zoom);
        }

        for zone in layout.layer_zone_nodes(0) {
            painter.paint_polygon(
                &layout.zone(zone).shape().polygon,
                ColorU::new(200, 52, 52, 255),
                view.zoom,
            );
        }

        for ghost in ghosts {
            painter.paint_primitive(&ghost, ColorU::new(75, 75, 150, 255), view.zoom);
        }

        if let Some(ref navmesh) = maybe_navmesh {
            for edge in navmesh.graph().edge_references() {
                let from =
                    PrimitiveIndex::from(navmesh.graph().node_weight(edge.source()).unwrap().node)
                        .primitive(layout.drawing())
                        .shape()
                        .center();
                let to =
                    PrimitiveIndex::from(navmesh.graph().node_weight(edge.target()).unwrap().node)
                        .primitive(layout.drawing())
                        .shape()
                        .center();

                let color = 'blk: {
                    if let (Some(source_pos), Some(target_pos)) = (
                        path.iter().position(|node| *node == edge.source()),
                        path.iter().position(|node| *node == edge.target()),
                    ) {
                        if target_pos == source_pos + 1 {
                            break 'blk ColorU::new(250, 250, 0, 255);
                        }
                    }

                    ColorU::new(125, 125, 125, 255)
                };

                painter.paint_edge(from, to, color, view.zoom);
            }
        }

        let mut scene = SceneProxy::from_scene(
            canvas.into_canvas().into_scene(),
            renderer.mode().level,
            RayonExecutor,
        );
        scene.build_and_render(renderer, BuildOptions::default());
        window.gl_swap_window();

        i += 1;
        if times != -1 && i >= times {
            return;
        }

        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
