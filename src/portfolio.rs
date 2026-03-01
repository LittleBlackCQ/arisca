use crate::config::Config;
use clap::Parser;
use crate::{
    bipoly::Polynomial,
    circuit::Circuit,
    verifier::{ReductionStats, verify},
};

use log::{error, info};
use std::{
    ffi::{OsString},
    sync::{Arc, mpsc},
    thread::spawn,
};

pub struct Portfolio {
    configs: Vec<(String, Config)>,
}

impl Portfolio {
    pub fn new(cfg: Config) -> Self {
        let mut configs = Vec::new();
        let mut id = 0;
        let mut add_config = |args: &str| {
            let worker_name = format!("Worker{id}");
            info!("{} with args: {:?}", worker_name, args);
            id += 1;
            let mut arg_vec: Vec<OsString> = vec![
                "arisca".into(),
                cfg.path.clone().into()
            ];
            for arg in args.split_whitespace() {
                arg_vec.push(arg.into());
            }
            match Config::try_parse_from(arg_vec) {
                Ok(cfg) => {
                    configs.push((worker_name, cfg));
                }
                Err(e) => {
                    error!("Error parsing arguments {}: {}", args, e);
                }
            }
        };
        add_config("--max-ratio 0.01 -r 9");
        add_config("--max-ratio 0.01 -r 5");
        add_config("-m bfs -r 9");
        add_config("-m bfs -r 5");
        add_config("-m dfs -r 5");

        Self {
            // base_config: cfg,
            configs: configs,
        }
    }

    pub fn run(self, circuit: Arc<Circuit>) -> Result<(Polynomial, ReductionStats, String), String> {
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

pub fn portfolio_main(circuit: Circuit, cfg: Config, stats: &mut ReductionStats) -> Result<Polynomial, String> {
    info!("Starting portfolio...");
    let shared_circuit = Arc::new(circuit);
    let portfolio = Portfolio::new(cfg);
    match portfolio.run(shared_circuit) {
        Ok((poly, new_stats, name)) => {
            info!("{} finished first.", name);
            *stats = new_stats;
            Ok(poly)
        }
        Err(e) => {
            Err(e)
        }
    }
}
