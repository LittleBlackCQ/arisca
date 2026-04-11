use crate::config::Config;
use crate::{
    Result,
    bipoly::Polynomial,
    circuit::Circuit,
    verifier::{ReductionStats, verify},
};
use clap::Parser;

use log::info;
use std::{
    ffi::OsString,
    sync::{Arc, mpsc},
    thread::spawn,
};

pub struct Portfolio {
    configs: Vec<(String, Config)>,
}

impl Portfolio {
    pub fn new(_cfg: Config) -> Self {
        let mut configs = Vec::new();
        let mut id = 0;

        info!(
            "Base args: {:?}",
            std::env::args().skip(2).collect::<Vec<String>>().join(" ")
        );
        let base_args: Vec<OsString> = std::env::args_os().collect();
        let mut add_config = |args: &str| {
            let worker_name = format!("Worker{id}");
            info!("{} with args: {:?}", worker_name, args);
            id += 1;
            let mut arg_vec: Vec<OsString> = base_args.clone();
            for arg in args.split_whitespace() {
                arg_vec.push(arg.into());
            }
            match Config::try_parse_from(arg_vec) {
                Ok(mut cfg) => {
                    cfg.portfolio = false;
                    configs.push((worker_name, cfg));
                }
                Err(e) => {
                    e.exit();
                }
            }
        };
        add_config("-m heuristic -r 9");
        add_config("-m heuristic");
        add_config("-m heuristic --flip");
        add_config("-m heuristic --flip -r 10");
        add_config("-m heuristic --max-ratio 0.1 --flip -r 10");
        add_config("-m bfs");
        add_config("-m dfs");
        add_config("-m heuristic -r 10 --delay");
        add_config("-m heuristic -r 10 --delay --flip");

        Self {
            // base_config: cfg,
            configs: configs,
        }
    }

    pub fn run(self, circuit: Arc<Circuit>) -> Result<(Polynomial, ReductionStats, String)> {
        let total_workers = self.configs.len();
        if total_workers == 0 {
            return Err("No portfolio configurations available.".into());
        }

        let (tx, rx) = mpsc::channel();

        for (worker_name, cfg) in self.configs {
            let tx_clone = tx.clone();
            let circuit_clone = Arc::clone(&circuit);

            spawn(move || {
                let mut local_stats = ReductionStats::new();
                let result = verify(&circuit_clone, &cfg, &mut local_stats);
                let _ = tx_clone.send((result, local_stats, worker_name));
            });
        }

        drop(tx);
        let mut completed = 0;

        while let Ok((result, local_stats, worker_name)) = rx.recv() {
            completed += 1;

            match result {
                Ok(poly) => {
                    return Ok((poly, local_stats, worker_name));
                }
                Err(e) => {
                    info!("{} failed: {:?}", worker_name, e);
                    if completed == total_workers {
                        return Err("All portfolio workers failed.".into());
                    }
                }
            }
        }

        Err("Portfolio aborted unexpectedly.".into())
    }
}

pub fn portfolio_main(
    circuit: Circuit,
    cfg: Config,
    stats: &mut ReductionStats,
) -> Result<Polynomial> {
    info!("Starting portfolio...");
    let shared_circuit = Arc::new(circuit);
    let portfolio = Portfolio::new(cfg);
    match portfolio.run(shared_circuit) {
        Ok((poly, new_stats, name)) => {
            info!("{} finished first.", name);
            *stats = new_stats;
            Ok(poly)
        }
        Err(e) => Err(e),
    }
}
