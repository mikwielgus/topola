// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Default)]
#[command(about, version)]
pub struct Cli {
    #[arg(
        value_name = "SPECCTRA DESIGN FILE",
        help = "Specify the Specctra Design (*.dsn) input file for the Topola autorouter"
    )]
    pub input: PathBuf,
    #[arg(
        short,
        long,
        value_name = "SPECCTRA SESSION FILE",
        help = "Specify the output session file in Specctra-compatible format (*.ses). The input filename is used by default, with the extension changed to Specctra Session File extension"
    )]
    pub output: Option<PathBuf>,
    #[arg(
        short,
        long,
        value_name = "COMMAND FILE",
        help = "JSON-like file with .cmd extension, containing sequence of available commands "
    )]
    pub commands: Option<PathBuf>,
    #[arg(
        long,
        help = "Force multilayer autorouting (vias allowed). Default when the DSN has two or more copper layers unless --planar is set"
    )]
    pub multilayer: bool,
    #[arg(
        long,
        help = "Force single-layer planar autorouting on --principal-layer (default 0)"
    )]
    pub planar: bool,
    #[arg(
        long,
        default_value_t = 0,
        help = "Principal copper layer index for planar routing / planar portion of multilayer"
    )]
    pub principal_layer: usize,
    #[arg(
        long,
        value_name = "NET[,NET...]",
        help = "Only autoroute the given net names (comma-separated)"
    )]
    pub nets: Option<String>,
    #[arg(
        long,
        help = "Only autoroute still-open ratlines (skip already connected nets)"
    )]
    pub remaining: bool,
    #[arg(
        long,
        value_name = "WIDTH",
        help = "Override routed band width in design units (default: structure rule width from DSN)"
    )]
    pub band_width: Option<f64>,
    #[arg(
        long,
        value_name = "RADIUS",
        help = "Override via pad radius in design units (default: from DSN via padstack)"
    )]
    pub via_radius: Option<f64>,
    #[arg(
        long,
        help = "Allow ratline order permutation / rip-up retries (can loop for a long time on dense boards)"
    )]
    pub permutate: bool,
    #[arg(
        long,
        default_value = "GND,gnd",
        value_name = "NET[,NET...]",
        help = "Net names to skip (default: GND,gnd). Pass empty string to skip nothing"
    )]
    pub skip_nets: String,
    #[arg(
        long,
        default_value_t = 2.0,
        help = "Initial per-attempt timeout seconds (TimeVsProgressAccumulator)"
    )]
    pub timeout_initial: f64,
    #[arg(
        long,
        default_value_t = 0.0,
        help = "Progress bonus seconds (0 avoids timeout extending forever on dense boards)"
    )]
    pub timeout_progress_bonus: f64,
    #[arg(
        long,
        default_value_t = 120.0,
        help = "Hard wall-clock seconds for the whole autoroute job (abort and still write SES)"
    )]
    pub wall_timeout: f64,
}
