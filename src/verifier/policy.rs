use super::{ReductionEngine, SizeGuard, SizeLimitExceeded, normalize};
use crate::bipoly::{Polynomial, VarId};

use std::collections::HashMap;
use log::debug;

pub enum ReductionAction {
    Reduce(VarId),
    Replace(Polynomial, VarId),
    Stop
}

pub trait ReductionPolicy {
    fn next_action(&mut self, _engine: &mut ReductionEngine) -> ReductionAction;
}

pub struct DefaultPolicy;
impl ReductionPolicy for DefaultPolicy {
    fn next_action(&mut self, engine: &mut ReductionEngine) -> ReductionAction {
        if let Some(var) = engine.state.var_domain.candidates().last() {
            ReductionAction::Reduce(*var)
        } else {
            ReductionAction::Stop
        }
    }
}

pub struct LazyGreedyPolicy {
    rate_limit: f64,
    size_limit: usize,
    penalty: HashMap<VarId, u32>,
}

impl ReductionPolicy for LazyGreedyPolicy {
    fn next_action(&mut self, engine: &mut ReductionEngine) -> ReductionAction {
        let candidates = engine.state.var_domain.candidates();
        if candidates.len() == 0 {
            return ReductionAction::Stop;
        }
        let candidates = self.sort_queue_by_occ_penalty(candidates, &engine.state.poly, engine.ctx.poly_map);
        let current_size = engine.state.poly.size() as f64;

        let mut best_candidate: Option<(f64, (VarId, Polynomial))> = None;

        let guard = SizeGuard::new(self.size_limit * engine.state.poly.size());
        for &var in candidates.iter() { 
            match self.try_reduce_var(var, &guard, engine) {
                Ok(reduced_poly) => {
                    let rate = (reduced_poly.size() as f64 - current_size) / current_size;
                    if rate < self.rate_limit {
                        debug!("Choose var: {:?}", var);
                        return ReductionAction::Replace(reduced_poly, var);
                    } else {
                        let value = self.penalty.entry(var).or_insert(1);
                        *value = value.saturating_mul(2);
                        match best_candidate {
                            None => best_candidate = Some((rate, (var, reduced_poly))),
                            Some((best_rate, _)) => {
                                if rate < best_rate {
                                    best_candidate = Some((rate, (var, reduced_poly)));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    debug!("Limit {:?} exceeded when reduce Var {:?}", e.limit, var);
                }
            }
        }
        if let Some((_, (var, reduced_poly))) = best_candidate {
            debug!("Choose var: {:?}", var);
            return ReductionAction::Replace(reduced_poly, var);
        } else {
            return ReductionAction::Reduce(candidates[0]);
        }
    }
}

impl LazyGreedyPolicy {
    pub fn new(rate_limit: f64, size_limit: usize) -> Self {
        LazyGreedyPolicy {
            rate_limit,
            size_limit,
            penalty: HashMap::new(),
        }
    }

    fn try_reduce_var(&mut self, var: VarId, guard: &SizeGuard, engine: &mut ReductionEngine) -> Result<Polynomial, SizeLimitExceeded> {
        let mut reduced_poly = engine.state.poly.clone();
        let Some(gate_poly) = engine.ctx.poly_map.get(&var) else { return Ok(reduced_poly); };
        debug!("Try reduce var: {:?} with size: {:?}", var, gate_poly.size());
        let factor = engine.state.poly.divide_by_term(&[var]);
        reduced_poly.sub_assign_checked(&(gate_poly * factor), guard)?;
        normalize(&mut reduced_poly, engine.ctx.modulus);

        debug!("Polynomial size after try reduce: {:?}", reduced_poly.size());

        Ok(reduced_poly)
    }

    fn sort_queue_by_occ_penalty(&mut self, candidates: &[VarId], poly: &Polynomial, poly_map: &HashMap<VarId, Polynomial>) -> Vec<VarId> {
        let mut stats: Vec<(_, u32)> = candidates.iter().map(|&v| (v, 0)).collect();
        stats.sort_by_key(|(v, _)| *v);

        let min_q = stats.first().unwrap().0;
        let max_q = stats.last().unwrap().0;

        for mono in poly.terms() {
            let vars = mono.term();

            if vars.is_empty() || *vars.last().unwrap() < min_q {
                continue;
            }
            if vars[0] > max_q {
                break;
            }

            let (mut i, mut j) = (0, 0);
            while i < stats.len() && j < vars.len() {
                let target = stats[i].0;
                let current = vars[j];

                if target == current {
                    stats[i].1 += 1;
                    i += 1;
                    j += 1;
                } else if current < target {
                    j += 1;
                } else {
                    i += 1;
                }
            }
        }

        stats.sort_by_key(|(v, count)| {
            count * 
            poly_map.get(v).unwrap_or(&Polynomial::zero()).size() as u32 *
            *self.penalty.entry(*v).or_insert(1)
        });
        stats.into_iter().map(|(v, _)| v).collect()
    }
}
