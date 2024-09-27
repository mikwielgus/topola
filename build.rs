include!("src/bin/topola/cli.rs");
use clap::CommandFactory;
use clap_mangen::Man;
use std::fs::{create_dir_all, File};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cmd = Cli::command();
    let man = Man::new(cmd);
    let folder = "man";
    create_dir_all(folder)?;
    let mut file = File::create(format!("{}/topola.1", folder))?;
    man.render(&mut file)?;
    Ok(())
}
