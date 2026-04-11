use log::info;

use arisca::{
    Result,
    aiger::AigerParser,
    circuit::{AdderExtractor, GenericExtractor, MajExtractor, Xor3Extractor, XorExtractor},
    config::{Config, ExtractMode, init_logger},
    portfolio::portfolio_main,
    verifier::{ReductionStats, verify},
};

fn main() -> Result<()> {
    let cfg = Config::parse_args();
    init_logger(cfg.log_file.as_ref());

    let mut circuit = AigerParser::from_aig(&cfg.path)?;

    for mode in &cfg.extract {
        match mode {
            ExtractMode::Adder => {
                circuit = AdderExtractor::run(&circuit);
            }
            ExtractMode::Xor => {
                circuit = GenericExtractor::run(&circuit, XorExtractor);
            }
            ExtractMode::Maj => {
                circuit = GenericExtractor::run(&circuit, MajExtractor);
            }
            ExtractMode::Xor3 => {
                circuit = GenericExtractor::run(&circuit, Xor3Extractor);
            }
        }
    }

    info!("Verifying circuit file {:?}", cfg.path);

    let mut stats = ReductionStats::new();
    let result_poly = if cfg.portfolio {
        portfolio_main(circuit, cfg, &mut stats)?
    } else {
        verify(&circuit, &cfg, &mut stats)?
    };
    if result_poly.is_zero() {
        info!("Verification success!");
    } else {
        info!(
            "Verification failed! Residue polynomial: {:?}.",
            result_poly
        )
    }
    info!("{:?}", stats);
    Ok(())
}
