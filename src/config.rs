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

#[derive(Clone, Debug, ValueEnum)]
pub enum ExtractMode {
    Adder,
    Xor,
    Maj,
    Xor3,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    #[arg(value_name = "AIG_FILE")]
    pub path: PathBuf,

    #[arg(short, long)]
    pub log_file: Option<PathBuf>,

    #[arg(long)]
    pub meta_file: Option<PathBuf>,

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

    #[arg(long, default_value_t = false, help = "Whether to delay the reduction of cone polynomial during the main reduction")]
    pub delay: bool,

    #[arg(long, default_value_t = false, help = "Enable portfolio mode")]
    pub portfolio: bool,

    #[arg(long, default_value_t = false, help = "Whether to eliminate the size of the polynomial in the sort of candidate variables")]
    pub no_size_sort: bool,

    #[arg(short, long, value_enum, default_value_t = ReductionMode::Heuristic, help = "Reduction mode")]
    pub mode: ReductionMode,

    #[arg(short = 'r', value_parser = value_parser!(u8).range(0..=10), default_value_t = 5, help = "Sensitity to identify converging cones based on the ratio of half adders")]
    pub revsca_sensitivity: u8,

    #[arg(short = 'e', long = "extract", value_enum, use_value_delimiter = true, default_values_t = vec![ExtractMode::Adder], help = "Apply extraction strategy (single) or strategies in sequence (e.g., -e xor,adder,xor3). Default: adder")]
    pub extract: Vec<ExtractMode>,
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