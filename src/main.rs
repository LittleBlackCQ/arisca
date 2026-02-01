use std::time::Instant;
use log::info;

use arisca::{
    config::{Config, init_logger},
    aiger::AigerParser,
    verifier::{verify, ReductionStats},
    circuit::AdderExtractor,
};

fn main() -> Result<(), String> {
    let cfg = Config::parse_args();
    init_logger(cfg.log_file.as_ref());

    let mut stats = ReductionStats::default();

    let circuit_adder = AdderExtractor::run(&AigerParser::from_aig(&cfg.path)?);
    info!("Verifying circuit file {:?}", cfg.path);

    let start_time = Instant::now();

    let result_poly = verify(&circuit_adder, &cfg, &mut stats)?;
    if result_poly.is_zero() {
        info!("Verification successful in {:?}", start_time.elapsed());
    } else {
        info!("Verification failed, residue polynomial: {:?}.", result_poly)
    }
    info!("{:?}", stats);
    Ok(())
}
