use std::fs::File;
use env_logger::{Builder, Env, Target};

use arisca::{
    config::Config,
    aiger::AigerParser,
    bipoly::{RevscaStrategy, MultiplierSpec, PolyVerifier},
    circuit::AdderExtractor,
};

fn main() {
    let cfg = Config::parse_args();

    init_logger(&cfg);

    let circuit = AigerParser::from_aig(&cfg.path).expect("Failed to parse AIGER file");
    let circuit_adder = AdderExtractor::run(&circuit);

    if let Some(dot_file) = &cfg.dot_file {
        circuit_adder.to_dot(dot_file);
    }

    PolyVerifier::verify(&circuit_adder, MultiplierSpec, RevscaStrategy::default());
}

fn init_logger(cfg: &Config) {
    let mut builder = Builder::from_env(Env::default().default_filter_or("info"));
    
    builder.format_timestamp(None).format_target(false);

    if let Some(log_path) = &cfg.log_file {
        let file = File::create(log_path).expect("Unable to create log file.");
        builder.target(Target::Pipe(Box::new(file)));
    } else {
        builder.target(Target::Stdout);
    }

    builder.init();
}
