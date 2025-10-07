// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use clap::Parser;
use std::fs::File;
use std::io::BufReader;
use topola::autorouter::anterouter::AnterouterOptions;
use topola::autorouter::execution::Command;
use topola::autorouter::history::History;
use topola::autorouter::invoker::Invoker;
use topola::autorouter::selection::PinSelection;
use topola::autorouter::Autorouter;
use topola::autorouter::AutorouterOptions;
use topola::autorouter::PresortBy;
use topola::board::edit::BoardEdit;
use topola::router::RouterOptions;
use topola::specctra::design::SpecctraDesign;

pub mod cli;
use cli::Cli;

fn main() -> Result<(), std::io::Error> {
    let args = Cli::parse();
    let design_file = File::open(&args.input)?;
    let design_bufread = BufReader::new(design_file);

    let design =
        SpecctraDesign::load(design_bufread).expect("File failed to parse as Specctra DSN");

    let board = design.make_board(&mut BoardEdit::new());

    let history = if let Some(commands_filename) = args.commands {
        let command_file = File::open(commands_filename)?;
        let commands_bufread = BufReader::new(command_file);
        serde_json::from_reader(commands_bufread)?
    } else {
        let mut history = History::new();
        history.do_(
            Command::Autoroute(
                PinSelection::new_select_layer(&board, 0),
                AutorouterOptions {
                    presort_by: PresortBy::RatlineIntersectionCountAndLength,
                    permutate: true,
                    router: RouterOptions {
                        wrap_around_bands: true,
                        squeeze_through_under_bends: false,
                        routed_band_width: 100.0,
                    },
                },
            ),
            None,
        );
        history
    };

    let mut invoker = Invoker::new(Autorouter::new(board).unwrap());
    invoker.replay(history);

    let output_filename = args
        .output
        .unwrap_or_else(|| args.input.clone().with_extension("ses"));
    let mut file = File::create(output_filename).unwrap();
    design
        .write_ses(invoker.autorouter().board(), &mut file)
        .expect("Failed to write Specctra Session file");

    Ok(())
}
