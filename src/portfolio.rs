use crate::config::Config;
use crate::{
    Result,
    bipoly::Polynomial,
    circuit::Circuit,
    verifier::{ReductionStats, verify_with_cancel},
};
use clap::{Parser, ValueEnum};

use log::info;
use serde::Deserialize;
use std::{
    fs,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::spawn,
};

pub struct Portfolio {
    configs: Vec<(String, Config)>,
}

#[derive(Deserialize)]
struct PortfolioConfig {
    workers: Vec<String>,
}

fn value_name(value: &impl ValueEnum) -> String {
    value.to_possible_value().unwrap().get_name().to_owned()
}

fn base_args(cfg: &Config) -> Vec<String> {
    let mut args = vec![
        "worker".into(),
        cfg.path.to_string_lossy().into_owned(),
        format!("--max-ratio={}", cfg.max_ratio),
        format!("--size-limit={}", cfg.size_limit),
        format!("--mode={}", value_name(&cfg.mode)),
        format!("--cone-expansion={}", value_name(&cfg.cone_expansion)),
        format!(
            "--extract={}",
            cfg.extract
                .iter()
                .map(value_name)
                .collect::<Vec<_>>()
                .join(",")
        ),
    ];
    args.extend(
        [
            cfg.log_file
                .as_ref()
                .map(|path| format!("--log-file={}", path.to_string_lossy())),
            cfg.meta_file
                .as_ref()
                .map(|path| format!("--meta-file={}", path.to_string_lossy())),
            cfg.spec_str.as_ref().map(|spec| format!("--spec={spec}")),
            cfg.signed.then(|| "--signed".into()),
        ]
        .into_iter()
        .flatten(),
    );
    args
}

impl Portfolio {
    pub fn new(cfg: Config) -> Result<Self> {
        let path = cfg.portfolio_config.as_deref().unwrap_or(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/portfolio.toml"
        )));
        let source = fs::read_to_string(path)?;
        let workers: PortfolioConfig = toml::from_str(&source)?;
        let base_args = base_args(&cfg);
        let configs = workers
            .workers
            .into_iter()
            .enumerate()
            .map(|(id, args)| {
                let words = args.split_whitespace().map(str::to_owned);
                let mut worker = Config::try_parse_from(base_args.iter().cloned().chain(words))?;
                if worker.path != cfg.path
                    || worker.spec_str != cfg.spec_str
                    || worker.signed != cfg.signed
                    || worker.extract != cfg.extract
                {
                    return Err(
                        "portfolio workers cannot change path, spec, signed, or extract".into(),
                    );
                }
                worker.portfolio = false;
                worker.portfolio_config = None;
                let name = format!("Worker{id}");
                info!("{}: {}", name, args);
                Ok((name, worker))
            })
            .collect::<Result<_>>()?;
        Ok(Self { configs })
    }

    pub fn run(self, circuit: Arc<Circuit>) -> Result<(Polynomial, ReductionStats, String)> {
        let total_workers = self.configs.len();
        if total_workers == 0 {
            return Err("No portfolio configurations available.".into());
        }

        let (tx, rx) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut handles = Vec::with_capacity(total_workers);

        for (worker_name, cfg) in self.configs {
            let tx_clone = tx.clone();
            let circuit_clone = Arc::clone(&circuit);
            let cancelled_clone = Arc::clone(&cancelled);

            handles.push(spawn(move || {
                let mut local_stats = ReductionStats::new();
                let result = verify_with_cancel(
                    &circuit_clone,
                    &cfg,
                    &mut local_stats,
                    Some(cfg.size_limit),
                    Some(&cancelled_clone),
                );
                let _ = tx_clone.send((result, local_stats, worker_name));
            }));
        }

        drop(tx);
        let mut completed = 0;

        let outcome = loop {
            let Ok((result, local_stats, worker_name)) = rx.recv() else {
                break Err("Portfolio aborted unexpectedly.".into());
            };
            completed += 1;

            match result {
                Ok(poly) => {
                    break Ok((poly, local_stats, worker_name));
                }
                Err(e) => {
                    info!("{} failed: {:?}", worker_name, e);
                    if completed == total_workers {
                        break Err("All portfolio workers failed.".into());
                    }
                }
            }
        };

        cancelled.store(true, Ordering::Relaxed);
        for handle in handles {
            let _ = handle.join();
        }
        outcome
    }
}

pub fn portfolio_main(
    circuit: Circuit,
    cfg: Config,
    stats: &mut ReductionStats,
) -> Result<Polynomial> {
    info!("Starting portfolio...");
    let shared_circuit = Arc::new(circuit);
    let portfolio = Portfolio::new(cfg)?;
    match portfolio.run(shared_circuit) {
        Ok((poly, new_stats, name)) => {
            info!("{} finished first.", name);
            *stats = new_stats;
            Ok(poly)
        }
        Err(e) => Err(e),
    }
}
