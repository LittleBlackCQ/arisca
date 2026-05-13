use log::info;

use arisca::{
    Result,
    config::{Config, init_logger},
};

fn main() -> Result<()> {
    let cfg = Config::parse_args();
    init_logger(cfg.log_file.as_ref());

    info!("Verifying circuit file {:?}", cfg.path);

    let result_poly = arisca::run(cfg)?;
    if result_poly.is_zero() {
        info!("Verification success!");
    } else {
        info!(
            "Verification failed! Residue polynomial: {:?}.",
            result_poly
        )
    }
    Ok(())
}
