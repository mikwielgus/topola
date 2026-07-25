// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use clap::Parser;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::BufReader;
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
use topola::stepper::TimeoutOptions;

pub mod cli;
use cli::Cli;

fn default_command(design: &SpecctraDesign, args: &Cli) -> Command {
    let board = design.make_board(&mut BoardEdit::new());
    let mut probe = Autorouter::new(board).expect("Failed to build ratsnest");

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
        let net_names: BTreeSet<String> = nets
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        selection = selection.filter_by_net_names(probe.board(), &net_names);
    }

    let planar_options = PlanarAutorouteOptions {
        principal_layer: args.principal_layer,
        presort_by: PresortBy::RatlineIntersectionCountAndLength,
        permutate: true,
        router: RouterOptions {
            wrap_around_bands: true,
            squeeze_through_under_bends: false,
            routed_band_width: band_width,
        },
        timeout: TimeoutOptions {
            initial: 1.0,
            progress_bonus: 0.005,
        },
    };

    eprintln!(
        "topola: layers={} multilayer={} band_width={} via_radius={} remaining={} nets={} pins={}",
        design.layer_count(),
        use_multilayer,
        band_width,
        via_radius,
        args.remaining,
        args.nets.as_deref().unwrap_or("*"),
        selection.selectors().count(),
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
                    initial: 5.0,
                    progress_bonus: 0.5,
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
    invoker.replay(history);

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
