
use super::{ReductionPolicy, ReductionContext, ReductionAction, ReductionState, ReductionStats, FlipManager, normalize};
use crate::bipoly::{Polynomial, VarId};

use log::debug;

pub struct ReductionEngine<'a> {
    pub ctx: &'a ReductionContext<'a>,
    pub stats: Option<&'a mut ReductionStats>,
    pub state: ReductionState,
    pub flip_manager: Option<FlipManager>,
}

impl<'a> ReductionEngine<'a> {

    pub fn new(ctx: &'a ReductionContext, state: ReductionState, stats: Option<&'a mut ReductionStats>, flip: bool) -> Self {
        let flip_manager = if flip { Some(FlipManager::new()) } else { None };
        Self { ctx, state, stats, flip_manager }
    }

    pub fn is_flip(&self, var: &VarId) -> bool {
        if let Some(flip_manager) = &self.flip_manager {
            flip_manager.is_flip(var)
        } else {
            false
        }
    }

    pub fn reduce_var(&mut self, var: VarId) {
        let Some(gate_poly) = self.ctx.poly_map.get(&var) else { return; };
        debug!("Reduce var: {:?} with size: {:?}", var, gate_poly.size());

        let factor = self.state.poly.divide_by_var(&var);
        if factor.is_zero() { return; }

        self.state.poly.substitute_by_poly(&var, gate_poly, self.is_flip(&var));
        normalize(&mut self.state.poly, self.ctx.modulus);
        debug!("Polynomial size after reduce: {:?}", self.state.poly.size());
    }

    pub fn run<P: ReductionPolicy>(mut self, policy: &mut P) -> Polynomial {

        loop {
            match policy.next_action(&mut self) {
                ReductionAction::Stop => break,
                ReductionAction::Reduce(var) => {
                    self.reduce_var(var);
                    self.state.var_domain.update(var);
                }
                ReductionAction::Replace(poly, var) => {
                    self.state.poly = poly;
                    self.state.var_domain.update(var);
                }
            }

            if let Some(flip_manager) = &mut self.flip_manager {
                flip_manager.greedy_flip(&mut self.state.poly, self.state.var_domain.candidates());
            }
            if let Some(stats) = self.stats.as_mut() {
                stats.update_size(self.state.poly.size());
            }
        }
        self.state.poly
    }
}    
