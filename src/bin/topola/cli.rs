use clap::Parser;
use std::path::PathBuf;


#[derive(Parser, Debug, Default)]
#[command(about, version)]
pub struct Cli {
    #[arg(value_name = "SPECCTRA DESIGN FILE",
	  help = "Specify the Specctra Design (*.dsn) input file for the Topola autorouter")]
    pub input: PathBuf,
    #[arg(short, long, value_name = "SPECCTRA SESSION FILE",
	  help = "Specify the output session file in Specctra-compatible format (*.ses). The input filename is used by default, with the extension changed to Specctra Session File extension")
    ]
    pub output: Option<PathBuf>,
    #[arg(short, long, value_name = "COMMAND FILE", help = "JSON-like file with .cmd extension, containing sequence of available commands ")]
    pub commands: Option<PathBuf>,
}
