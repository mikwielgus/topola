use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use clap::{Parser, Error};

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
            topola::autorouter::Autorouter::new(board).unwrap());
        invoker.replay(serde_json::from_reader(history_bufread).unwrap())
    }
    
    Ok(())
    // let content = std::fs::read_to_string(&args.input).expect("could not read file");

    // for line in content.lines() {
    //     if line.contains(&args.pattern) {
    //         println!("{}", line);
    //     }
    // }
} 