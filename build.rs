include!("src/bin/topola/cli.rs");
use clap_mangen::Man;
use clap::CommandFactory;
use std::fs::{File, create_dir_all};
// https://rust-cli.github.io/book/in-depth/docs.html
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cmd = Cli::command();
    let man = Man::new(cmd);
    let folder = "man";
    create_dir_all(folder)?;
    let mut file = File::create(format!("{}/topola.1", folder))?;
    man.render(&mut file)?;
    Ok(())
}
