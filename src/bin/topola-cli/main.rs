use clap::{Error, Parser};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

#[derive(Parser)]
struct Cli {
    input: std::path::PathBuf,
    output: std::path::PathBuf,
    history: Option<std::path::PathBuf>,
}

fn main() -> Result<(), std::io::Error> {
    let args = Cli::parse();

    let design_file = File::open(args.input)?;
    let mut design_bufread = BufReader::new(design_file);

    let design = topola::specctra::design::SpecctraDesign::load(design_bufread).unwrap();
    let board = design.make_board();

    if let Some(history) = args.history {
        let history_file = File::open(history)?;
        let mut history_bufread = BufReader::new(history_file);
        let mut invoker = topola::autorouter::invoker::Invoker::new(
            topola::autorouter::Autorouter::new(board).unwrap(),
        );
        invoker.replay(serde_json::from_reader(history_bufread).unwrap())
    }

    Ok(())
}
