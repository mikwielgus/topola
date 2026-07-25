// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use clap::Parser;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::BufReader;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use topola::autorouter::anterouter::AnterouterOptions;
use topola::autorouter::execution::Command;
use topola::autorouter::history::History;
use topola::autorouter::invoker::Invoker;
use topola::autorouter::multilayer_autoroute::MultilayerAutorouteOptions;
use topola::autorouter::selection::PinSelection;
use topola::autorouter::Autorouter;
use topola::autorouter::PlanarAutorouteOptions;
use topola::autorouter::PresortBy;
use topola::board::edit::BoardEdit;
use topola::router::RouterOptions;
use topola::specctra::design::SpecctraDesign;
use topola::stepper::{Abort, Step, TimeoutOptions};

pub mod cli;
use cli::Cli;

fn parse_net_list(s: &str) -> BTreeSet<String> {
    s.split(',')
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(str::to_string)
        .collect()
}

fn default_command(design: &SpecctraDesign, args: &Cli) -> Command {
    let board = design.make_board(&mut BoardEdit::new());
    let probe = Autorouter::new(board).expect("Failed to build ratsnest");

    let band_width = args
        .band_width
        .unwrap_or_else(|| design.default_trace_width());
    let via_radius = args
        .via_radius
        .unwrap_or_else(|| design.default_via_radius());
    let use_multilayer = if args.planar {
        false
    } else {
        args.multilayer || design.layer_count() >= 2
    };

    let mut selection = if args.remaining {
        probe.pin_selection_for_unconnected_ratlines()
    } else if use_multilayer {
        PinSelection::new_select_all_layers(probe.board())
    } else {
        PinSelection::new_select_layer(probe.board(), args.principal_layer)
    };

    if let Some(nets) = &args.nets {
        selection = selection.filter_by_net_names(probe.board(), &parse_net_list(nets));
    }

    let skip = parse_net_list(&args.skip_nets);
    if !skip.is_empty() {
        selection = selection.exclude_net_names(probe.board(), &skip);
    }

    let planar_options = PlanarAutorouteOptions {
        principal_layer: args.principal_layer,
        presort_by: PresortBy::RatlineIntersectionCountAndLength,
        // Default OFF — permutation/reconfigure loops are what hang dense KiCad boards.
        permutate: args.permutate,
        router: RouterOptions {
            wrap_around_bands: true,
            squeeze_through_under_bends: false,
            routed_band_width: band_width,
        },
        timeout: TimeoutOptions {
            initial: args.timeout_initial,
            progress_bonus: args.timeout_progress_bonus,
        },
    };

    eprintln!(
        "topola: layers={} multilayer={} band_width={} via_radius={} remaining={} permutate={} pins={} wall_timeout={}s",
        design.layer_count(),
        use_multilayer,
        band_width,
        via_radius,
        args.remaining,
        args.permutate,
        selection.selectors().count(),
        args.wall_timeout,
    );

    if use_multilayer {
        Command::MultilayerAutoroute(
            selection,
            MultilayerAutorouteOptions {
                anterouter: AnterouterOptions {
                    fanout_clearance: band_width.max(200.0),
                    via_radius,
                },
                planar: planar_options,
                timeout: TimeoutOptions {
                    initial: args.timeout_initial.max(5.0),
                    progress_bonus: args.timeout_progress_bonus,
                },
            },
        )
    } else {
        Command::Autoroute(selection, planar_options)
    }
}

fn main() -> Result<(), std::io::Error> {
    let args = Cli::parse();
    let design_file = File::open(&args.input)?;
    let design_bufread = BufReader::new(design_file);

    let design =
        SpecctraDesign::load(design_bufread).expect("File failed to parse as Specctra DSN");

    let history = if let Some(commands_filename) = &args.commands {
        let command_file = File::open(commands_filename)?;
        let commands_bufread = BufReader::new(command_file);
        serde_json::from_reader(commands_bufread)?
    } else {
        let mut history = History::new();
        history.do_(default_command(&design, &args), None);
        history
    };

    let board = design.make_board(&mut BoardEdit::new());
    let mut invoker = Invoker::new(Autorouter::new(board).unwrap());

    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = stop.clone();
    let wall = Duration::from_secs_f64(args.wall_timeout.max(1.0));
    let started = Instant::now();

    // Drive execute_stepper with a wall clock so dense navmesh jobs cannot hang forever.
    let (done, undone) = history.dissolve();
    for entry in done {
        let command = entry.command().clone();
        match invoker.execute_stepper(command) {
            Ok(mut stepper) => {
                let mut steps: u64 = 0;
                loop {
                    if stop_flag.load(Ordering::Relaxed) || started.elapsed() >= wall {
                        eprintln!(
                            "topola: wall timeout after {:.1}s ({} steps) — writing partial SES",
                            started.elapsed().as_secs_f64(),
                            steps
                        );
                        stepper.abort(&mut invoker);
                        break;
                    }
                    match stepper.step(&mut invoker) {
                        Ok(std::ops::ControlFlow::Break(msg)) => {
                            eprintln!("topola: {}", msg);
                            break;
                        }
                        Ok(std::ops::ControlFlow::Continue(())) => {
                            steps += 1;
                            if steps % 5000 == 0 {
                                eprintln!(
                                    "topola: … {} steps, {:.1}s",
                                    steps,
                                    started.elapsed().as_secs_f64()
                                );
                            }
                        }
                        Err(err) => {
                            eprintln!("topola: step error (continuing to write SES): {err}");
                            break;
                        }
                    }
                }
            }
            Err(err) => {
                eprintln!("topola: execute_stepper error: {err}");
            }
        }
        if started.elapsed() >= wall {
            break;
        }
    }
    let _ = undone;
    let _ = stop;

    // Keep watchdog thread from being optimized out in future refactors.
    let _watch = thread::spawn(move || {
        thread::sleep(wall);
    });

    let output_filename = args
        .output
        .unwrap_or_else(|| args.input.clone().with_extension("ses"));
    let mut file = File::create(&output_filename).unwrap();
    design
        .write_ses(invoker.autorouter().board(), &mut file)
        .expect("Failed to write Specctra Session file");
    eprintln!("topola: wrote {}", output_filename.display());

    Ok(())
}
