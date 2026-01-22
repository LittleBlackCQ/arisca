use std::fs::File;
use env_logger::{Builder, Env, Target};

use arisca::{
    config::Config,
    aiger::AigerParser,
    bipoly::{RevscaStrategy, PolyVerifier, ArithmeticSpec},
    circuit::AdderExtractor,
};

fn main() -> Result<(), String> {
    let cfg = Config::parse_args();

    init_logger(&cfg);

    let circuit = AigerParser::from_aig(&cfg.path)?;
    let circuit_adder = AdderExtractor::run(&circuit);
    let spec = ArithmeticSpec::new(cfg.spec_str.as_deref(), cfg.signed)?;
    PolyVerifier::verify(&circuit_adder, spec, RevscaStrategy::default());
    Ok(())
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
