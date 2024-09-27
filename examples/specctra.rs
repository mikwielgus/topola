use std::fs::File;
use std::io::BufReader;
use topola::autorouter::history::History;
use topola::autorouter::invoker::Command;
use topola::autorouter::invoker::Invoker;
use topola::autorouter::selection::PinSelection;
use topola::autorouter::Autorouter;
use topola::specctra::design::SpecctraDesign;

fn main() -> Result<(), std::io::Error> {
    let design_file = File::open("example.dsn")?;
    let mut design_bufread = BufReader::new(design_file);

    let design = SpecctraDesign::load(design_bufread).unwrap();
    let board = design.make_board();
    
    let mut invoker = Invoker::new(Autorouter::new(board).unwrap());
    
    let mut file = File::create("example.ses").unwrap();
    design.write_ses(invoker.autorouter().board(), &mut file);

    let filename = design.get_name();
    Ok(())
}
