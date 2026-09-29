pub mod aiger;
pub mod bipoly;
pub mod circuit;
pub mod config;
pub mod json;
pub mod portfolio;
pub mod verifier;

use std::error::Error;
use std::time::Instant;
pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync + 'static>>;

use aiger::AigerParser;
use bipoly::Polynomial;
use circuit::{AdderExtractor, GenericExtractor, MajExtractor, Xor3Extractor, XorExtractor};
use config::{Config, ExtractMode};
use portfolio::portfolio_main;
use verifier::verify;

pub fn run(cfg: Config) -> Result<Polynomial> {
    let start_time = Instant::now();
    let mut circuit = AigerParser::from_aig(&cfg.path)?;

    for mode in &cfg.extract {
        match mode {
            ExtractMode::Adder => circuit = AdderExtractor::run(&circuit),
            ExtractMode::Xor => circuit = GenericExtractor::run(&circuit, XorExtractor),
            ExtractMode::Maj => circuit = GenericExtractor::run(&circuit, MajExtractor),
            ExtractMode::Xor3 => circuit = GenericExtractor::run(&circuit, Xor3Extractor),
        }
    }

    let state = if cfg.portfolio || cfg.portfolio_config.is_some() {
        portfolio_main(circuit, cfg)?
    } else {
        verify(&circuit, &cfg, None)?
    };
    log::info!(
        "Execution Summary:\n    - {:.<25} {}\n    - {:.<25} {:?}",
        "Max Poly Size",
        state.poly_sizes.iter().copied().max().unwrap_or_default(),
        "Total Time",
        start_time.elapsed(),
    );
    Ok(state.poly)
}
