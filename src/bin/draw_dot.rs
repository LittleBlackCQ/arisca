use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use env_logger::{Builder, Env, Target};

use arisca::{aiger::AigerParser, circuit::AdderExtractor, circuit::extractor::{GenericExtractor, XorExtractor, MajExtractor, Xor3Extractor}};

#[derive(Clone, ValueEnum)]
enum ExtractMode {
    Adder,
    Xor,
    Maj,
    Xor3
}

#[derive(Parser)]
#[command(name = "draw_dot")]
struct Config {
    #[arg(value_name = "AIG_FILE")]
    path: PathBuf,

    #[arg(short = 'e', long = "extract", value_enum)]
    extract_mode: Option<ExtractMode>,

    #[arg(short, long, value_name = "DOT_FILE")]
    dot_file: Option<PathBuf>,
}

fn main() -> Result<(), String>{
    init_logger();
    let args = Config::parse();
    
    let mut circuit = AigerParser::from_aig(args.path)?;
    if let Some(mode) = args.extract_mode {
        match mode {
            ExtractMode::Adder => {
                circuit = AdderExtractor::run(&circuit);
            },
            ExtractMode::Xor => {
                circuit = GenericExtractor::run(&circuit, XorExtractor);
            },
            ExtractMode::Maj => {
                circuit = GenericExtractor::run(&circuit, MajExtractor)
            },
            ExtractMode::Xor3 => {
                circuit = GenericExtractor::run(&circuit, Xor3Extractor)
            }
        }
    }
    if let Some(dot_file) = args.dot_file {
        circuit.to_dot(dot_file, None);
    }
    Ok(())
}

fn init_logger() {
    let mut builder = Builder::from_env(Env::default().default_filter_or("info"));
    builder.format_timestamp(None).format_target(false);
    builder.target(Target::Stdout);
    builder.init();
}