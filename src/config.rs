use clap::{Parser, ValueEnum, value_parser};
use std::path::PathBuf;
use std::fs::File;
use env_logger::{Builder, Env, Target};

#[derive(Clone, Debug, ValueEnum)]
pub enum ReductionMode {
    DFS,
    BFS,
    Random,
    Heuristic,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    #[arg(value_name = "AIG_FILE")]
    pub path: PathBuf,

    #[arg(short, long, value_name = "LOG_FILE")]
    pub log_file: Option<PathBuf>,

    #[arg(short = 's', long = "spec", value_name = "INPUT_SPEC")]
    pub spec_str: Option<String>,

    #[arg(long, help = "Whether the input spec is signed")]
    pub signed: bool,

    #[arg(long, default_value_t = 0.01, help = "Maximum size ratio between the reduced polynomial size and the original size")] 
    pub max_ratio: f64,

    #[arg(long, default_value_t = 5, help = "Abort if the size ratio between the reduced polynomial size and the original size is larger than this value")]
    pub abort_ratio: usize,

    #[arg(long, default_value_t = 1000000, help = "Polynomial size limit for main reduction")] 
    pub size_limit: usize,

    #[arg(long, default_value_t = false, help = "Enable dual variables")]
    pub flip: bool,

    #[arg(long, default_value_t = false, help = "Enable portfolio mode")]
    pub portfolio: bool,

    #[arg(short, long, value_enum, default_value_t = ReductionMode::Heuristic, help = "Reduction mode")]
    pub mode: ReductionMode,

    #[arg(short = 'r', value_parser = value_parser!(u8).range(0..=10), default_value_t = 5, help = "Sensitity to identify converging cones based on the ratio of half adders")]
    pub revsca_sensitivity: u8,
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
