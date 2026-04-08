use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use env_logger::{Builder, Env, Target};

use arisca::{
    aiger::{AigerParser, ToAig}, 
    json::ToJson,
    circuit::{AdderExtractor}, 
    circuit::extractor::{GenericExtractor, XorExtractor, MajExtractor, Xor3Extractor}
};

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

    #[arg(short = 'e', long = "extract", value_enum, use_value_delimiter = true, help = "Apply extraction strategy (single) or strategies in sequence (e.g., -e xor,adder,xor3)")]
    extract: Option<Vec<ExtractMode>>,

    #[arg(short, long, value_name = "DOT_FILE")]
    dot_file: Option<PathBuf>,

    #[arg(short = 'a', long = "aig", value_name = "OUTPUT_AIG")]
    aig_file: Option<PathBuf>,

    #[arg(short = 'j', long = "json", value_name = "OUTPUT_JSON")]
    json_file: Option<PathBuf>,
}

fn main() -> Result<(), String> {
    init_logger();
    let args = Config::parse();

    let mut circuit = AigerParser::from_aig(args.path)?;

    // Handle extraction chain (supports single or multiple strategies)
    if let Some(chain) = args.extract {
        for mode in chain {
            match mode {
                ExtractMode::Adder => {
                    circuit = AdderExtractor::run(&circuit);
                }
                ExtractMode::Xor => {
                    circuit = GenericExtractor::run(&circuit, XorExtractor);
                }
                ExtractMode::Maj => circuit = GenericExtractor::run(&circuit, MajExtractor),
                ExtractMode::Xor3 => circuit = GenericExtractor::run(&circuit, Xor3Extractor),
            }
        }
    }

    if let Some(dot_file) = args.dot_file {
        circuit.to_dot(&dot_file, None);
        log::info!("DOT file written to: {:?}", dot_file);
    }

    if let Some(aig_output_path) = args.aig_file {
        circuit
            .write_aig(&aig_output_path)
            .map_err(|e| format!("Failed to write AIG file: {}", e))?;

        log::info!("AIG file written to: {:?}", aig_output_path);
    }

    if let Some(json_output_path) = args.json_file {
        circuit
            .write_json(&json_output_path)
            .map_err(|e| format!("Failed to write JSON file: {}", e))?;

        log::info!("JSON file written to: {:?}", json_output_path);
    }
    Ok(())
}

fn init_logger() {
    let mut builder = Builder::from_env(Env::default().default_filter_or("info"));
    builder.format_timestamp(None).format_target(false);
    builder.target(Target::Stdout);
    builder.init();
}
