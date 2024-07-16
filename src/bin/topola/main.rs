use clap::{Error, Parser};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;
use topola::autorouter::history::History;
use topola::autorouter::invoker::Command;
use topola::autorouter::invoker::Invoker;
use topola::autorouter::selection::Selection;
use topola::autorouter::Autorouter;
use topola::specctra::design::SpecctraDesign;

#[derive(Parser, Debug, Default)]
#[command(about, version)]
struct Cli {
    input: PathBuf,
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,
    #[arg(short, long, value_name = "FILE")]
    commands: Option<PathBuf>,
}

fn main() -> Result<(), std::io::Error> {
    let args = Cli::parse();

    let design_file = File::open(&args.input)?;
    let mut design_bufread = BufReader::new(design_file);

    let design = SpecctraDesign::load(design_bufread).unwrap();
    let board = design.make_board();

    let history = if let Some(commands_filename) = args.commands {
        let command_file = File::open(commands_filename)?;
        let commands_bufread = BufReader::new(command_file);
        serde_json::from_reader(commands_bufread)?
    } else {
        let mut history = History::new();
        history.do_(Command::Autoroute(Selection::new_select_all(&board)));
        history
    };

    let mut invoker = Invoker::new(Autorouter::new(board).unwrap());
    invoker.replay(history);

    let output_filename = args
        .output
        .unwrap_or_else(|| args.input.clone().with_extension("ses"));
    let mut file = File::create(output_filename).unwrap();
    design.write_ses(invoker.autorouter().board(), &mut file);

    Ok(())
}
