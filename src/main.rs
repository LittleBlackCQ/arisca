use log::info;

use arisca::{
    config::{Config, init_logger},
    aiger::AigerParser,
    verifier::{verify, ReductionStats},
    circuit::AdderExtractor,
    portfolio::portfolio_main,
};

fn main() -> Result<(), String> {
    let cfg = Config::parse_args();
    init_logger(cfg.log_file.as_ref());

    let circuit_adder = AdderExtractor::run(&AigerParser::from_aig(&cfg.path)?);
    info!("Verifying circuit file {:?}", cfg.path);

    let mut stats = ReductionStats::new();
    let result_poly = if cfg.portfolio {
        portfolio_main(circuit_adder, cfg, &mut stats)?
    } else {
        verify(&circuit_adder, &cfg, &mut stats)?
    };
    if result_poly.is_zero() {
        info!("Verification success!");
    } else {
        info!("Verification failed! Residue polynomial: {:?}.", result_poly)
    }
    info!("{:?}", stats);
    Ok(())
}
