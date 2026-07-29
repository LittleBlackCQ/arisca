pub mod aiger;
pub mod bipoly;
pub mod circuit;
pub mod config;
pub mod json;
pub mod portfolio;
pub mod verifier;

use std::error::Error;
pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync + 'static>>;

use aiger::AigerParser;
use bipoly::Polynomial;
use circuit::{AdderExtractor, GenericExtractor, MajExtractor, Xor3Extractor, XorExtractor};
use config::{Config, ExtractMode};
use portfolio::portfolio_main;
use verifier::{ReductionStats, verify};

pub fn run(cfg: Config) -> Result<Polynomial> {
    let mut circuit = AigerParser::from_aig(&cfg.path)?;

    for mode in &cfg.extract {
        match mode {
            ExtractMode::Adder => circuit = AdderExtractor::run(&circuit),
            ExtractMode::Xor => circuit = GenericExtractor::run(&circuit, XorExtractor),
            ExtractMode::Maj => circuit = GenericExtractor::run(&circuit, MajExtractor),
            ExtractMode::Xor3 => circuit = GenericExtractor::run(&circuit, Xor3Extractor),
        }
    }

    let mut stats = ReductionStats::new();
    let result_poly = if cfg.portfolio {
        portfolio_main(circuit, cfg, &mut stats)?
    } else {
        verify(&circuit, &cfg, &mut stats, None)?
    };
    log::info!("{:?}", stats);
    Ok(result_poly)
}
