use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    #[arg(value_name = "AIG_FILE")]
    pub path: PathBuf,

    #[arg(short, long, value_name = "LOG_FILE")]
    pub log_file: Option<PathBuf>,
}

impl Config {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}