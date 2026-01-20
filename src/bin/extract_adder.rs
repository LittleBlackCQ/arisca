use clap::Parser;
use std::path::PathBuf;
use env_logger::{Builder, Env, Target};
use arisca::{aiger::AigerParser, circuit::AdderExtractor};

#[derive(Parser)]
#[command(name = "extract_adder")]
struct Config {
    #[arg(value_name = "AIG_FILE")]
    path: PathBuf,

    #[arg(short, long, value_name = "DOT_FILE")]
    dot_file: Option<PathBuf>,
}

fn main() {
    init_logger();
    let args = Config::parse();

    let circuit = AigerParser::from_aig(&args.path).expect("Failed to parse AIGER file");
    let circuit_adder = AdderExtractor::run(&circuit);

    if let Some(dot_path) = args.dot_file {
        circuit_adder.to_dot(&dot_path);
    }
}

fn init_logger() {
    let mut builder = Builder::from_env(Env::default().default_filter_or("info"));
    builder.format_timestamp(None).format_target(false);
    builder.target(Target::Stdout);
    builder.init();
}