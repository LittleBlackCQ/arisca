use clap::Parser;
use std::path::PathBuf;
use std::fs::File;
use env_logger::{Builder, Env, Target};

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

    #[arg(long, default_value_t = 0.1)] 
    pub max_ratio: f64,

    #[arg(long, default_value_t = 5)]
    pub abort_ratio: usize,
}

impl Config {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

pub fn init_logger(log_file: Option<&PathBuf>) {
    let mut builder = Builder::from_env(Env::default().default_filter_or("info"));
    
    builder.format_timestamp(None).format_target(false);

    if let Some(log_path) = log_file {
        let file = File::create(log_path).expect("Unable to create log file.");
        builder.target(Target::Pipe(Box::new(file)));
    } else {
        builder.target(Target::Stdout);
    }

    builder.init();
}
