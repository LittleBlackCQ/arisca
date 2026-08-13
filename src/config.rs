use clap::{Parser, ValueEnum};
use env_logger::{Builder, Env, Target};
use std::fs::File;
use std::path::PathBuf;

#[derive(Clone, Debug, ValueEnum)]
pub enum ReductionMode {
    DFS,
    BFS,
    Random,
    Heuristic,
}

#[derive(Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum ExtractMode {
    Adder,
    Xor,
    Maj,
    Xor3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ConeExpansion {
    None,
    Low,
    Medium,
    High,
}

impl ConeExpansion {
    pub const fn pair_ratio_threshold(self) -> u8 {
        match self {
            Self::None => 10,
            Self::Low => 9,
            Self::Medium => 5,
            Self::High => 0,
        }
    }
}

#[derive(Clone, Parser, Debug)]
#[command(version, about, long_about = None, args_override_self = true)]
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

    #[arg(
        long,
        default_value_t = 0.01,
        help = "Maximum size ratio between the reduced polynomial size and the original size"
    )]
    pub max_ratio: f64,

    #[arg(
        long,
        default_value_t = 1000000,
        help = "Polynomial size limit for portfolio workers (ignored in single-run mode)"
    )]
    pub size_limit: usize,

    #[arg(long, default_value_t = false, help = "Enable dual variables")]
    pub flip: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "Whether to delay the reduction of cone polynomial during the main reduction"
    )]
    pub delay: bool,

    #[arg(long, default_value_t = false, help = "Enable portfolio mode")]
    pub portfolio: bool,

    #[arg(long, value_name = "TOML_FILE", help = "Portfolio configuration")]
    pub portfolio_config: Option<PathBuf>,

    #[arg(short, long, value_enum, default_value_t = ReductionMode::Heuristic, help = "Reduction mode")]
    pub mode: ReductionMode,

    #[arg(
        long,
        value_enum,
        default_value_t = ConeExpansion::Medium,
        help = "Converging-cone expansion level based on the ratio of half-adder input pairs"
    )]
    pub cone_expansion: ConeExpansion,

    #[arg(short = 'e', long = "extract", value_enum, use_value_delimiter = true, default_values_t = vec![ExtractMode::Adder, ExtractMode::Xor3, ExtractMode::Maj, ExtractMode::Xor], help = "Apply extraction strategy (single) or strategies in sequence (e.g., -e xor,adder,xor3). Default: adder,xor3,maj,xor")]
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
