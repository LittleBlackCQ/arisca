use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    #[arg(value_name = "AIG_FILE")]
    pub path: PathBuf,

    #[arg(short, long, value_name = "LOG_FILE")]
    pub log_file: Option<PathBuf>,

    #[arg(short = 's', long = "spec", value_name = "INPUT_SPEC")]
    pub spec_str: Option<String>,

    #[arg(long = "signed")]
    pub signed: bool,
}

impl Config {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}