use std::path::PathBuf;

use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Opts {
    /// Enable tracing
    #[arg(long, env = "ENABLE_TRACING", default_value = "false")]
    pub tracing: bool,
    /// The path to the config file
    #[arg(long, default_value = "config.toml")]
    pub config_path: PathBuf,
}
