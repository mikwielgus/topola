// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::fs::File;
use std::io::BufReader;
use topola::autorouter::invoker::Invoker;
use topola::autorouter::Autorouter;
use topola::board::edit::BoardEdit;
use topola::specctra::design::SpecctraDesign;

fn main() -> Result<(), std::io::Error> {
    let design_file = File::open("example.dsn")?;
    let design_bufread = BufReader::new(design_file);

    let design = SpecctraDesign::load(design_bufread).unwrap();
    let board = design.make_board(&mut BoardEdit::new());

    let invoker = Invoker::new(Autorouter::new(board).unwrap());

    let mut file = File::create("example.ses").unwrap();
    design
        .write_ses(invoker.autorouter().board(), &mut file)
        .unwrap();

    //let filename = design.get_name();
    Ok(())
}
