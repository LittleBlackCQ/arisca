use super::{ReductionState, ReductionEngine, VarDomain, SizeGuard};
use crate::bipoly::{Polynomial, VarId};

use std::collections::HashMap;
use rand::{seq::SliceRandom, thread_rng};
use log::debug;

pub enum ReductionAction {
    Reduce(VarId),
    Replace(ReductionState),
    Skip,
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

pub struct RandomPolicy;
impl ReductionPolicy for RandomPolicy {
    fn next_action(&mut self, engine: &mut ReductionEngine) -> ReductionAction {
        if let Some(v) = engine.state.var_domain.candidates().choose(&mut thread_rng()) {
            ReductionAction::Reduce(*v)
        } else {
            ReductionAction::Stop
        }
    }

}

pub struct LazyGreedyPolicy {
    max_ratio: f64,
    abort_ratio: usize,
    penalty: HashMap<VarId, u32>,
}

impl ReductionPolicy for LazyGreedyPolicy {
    fn next_action(&mut self, engine: &mut ReductionEngine) -> ReductionAction {
        let candidates = engine.state.var_domain.candidates();
        if candidates.len() == 0 || engine.state.poly.size() == 0 {
            return ReductionAction::Stop;
        } else if candidates.len() == 1 {
            return ReductionAction::Reduce(candidates[0]);
        }

        let candidates = self.sort_queue_by_occ_penalty(&candidates, &engine.state.poly, engine.ctx.poly_map);
        let origin_state = engine.state.clone();
        let current_size = engine.state.poly.size();
        let mut best_candidate: Option<(f64, VarId, ReductionState)> = None;

        let guard = SizeGuard::new(self.abort_ratio * current_size);
        for &var in &candidates { 
            if let Err(err) = engine.reduce_var(var, Some(&guard)) {
                debug!("Var: {:?} failed. Error: {:?}", var, err);
                continue;
            }
            let ratio = (engine.state.poly.size() as f64 - current_size as f64) / current_size as f64;
            debug!("Try size: {:?}, ratio: {:.3}", engine.state.poly.size(), ratio);
            if ratio < self.max_ratio {
                return ReductionAction::Skip;
            } else {
                let value = self.penalty.entry(var).or_insert(1);
                *value = value.saturating_mul(2);
                match best_candidate {
                    None => best_candidate = Some((ratio, var, engine.state.clone())),
                    Some((best_ratio, _, _)) => {
                        if ratio < best_ratio {
                            best_candidate = Some((ratio, var, engine.state.clone()));
                        }
                    }
                }
                engine.state = origin_state.clone();
            }
        }
        if let Some((_, var, state)) = best_candidate {
            debug!("Choose var: {:?}", var);
            return ReductionAction::Replace(state);
        } else {
            return ReductionAction::Reduce(candidates[0]);
        }
    }
}

impl LazyGreedyPolicy {
    pub fn new(max_ratio: f64, abort_ratio: usize) -> Self {
        LazyGreedyPolicy {
            max_ratio,
            abort_ratio,
            penalty: HashMap::new(),
        }
    }

    fn sort_queue_by_occ_penalty(&mut self, candidates: &[VarId], poly: &Polynomial, poly_map: &HashMap<VarId, Polynomial>) -> Vec<VarId> {
        let mut stats: Vec<(_, u32)> = candidates.iter().map(|&v| (v, 0)).collect();
        stats.sort_by_key(|(v, _)| *v);

        let min_q = stats.first().unwrap().0;
        let max_q = stats.last().unwrap().0;

        for term in poly.terms.keys() {
            let vars = term.vars();
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
            poly_map.get(v).unwrap_or(&Polynomial::new()).size() as u32 *
            *self.penalty.entry(*v).or_insert(1)
        });
        stats.into_iter().map(|(v, _)| v).collect()
    }
}
