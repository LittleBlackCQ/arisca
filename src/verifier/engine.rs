
use super::{ReductionPolicy, ReductionContext, ReductionAction, ReductionState, ReductionStats, VarDomain, SizeGuard};
use crate::bipoly::{VarId};
use crate::Result;

use log::debug;

pub struct ReductionEngine<'a> {
    pub ctx: &'a ReductionContext<'a>,
    pub stats: Option<&'a mut ReductionStats>,
    pub state: ReductionState,
    pub size_limit: Option<usize>,
}

impl<'a> ReductionEngine<'a> {

    pub fn new(ctx: &'a ReductionContext, state: ReductionState, stats: Option<&'a mut ReductionStats>, size_limit: Option<usize>) -> Self {
        Self { ctx, state, stats, size_limit }
    }

    pub fn normalize(&mut self) {
        if let Some(modulus) = self.ctx.modulus {
            self.state.poly.mod_by_const(modulus);
        }
        self.state.poly.remove_mono_by(|m, _| {
            for pair in m.vars().windows(2) {
                let (var_i, var_j) = (pair[0], pair[1]);
                if var_j >= 0 {
                    return false;
                } else if (var_j - var_i) == 1 {
                    return true;
                } 
            }
            false
        });
    }

    pub fn reduce_var(&mut self, var: VarId, guard: Option<&SizeGuard>) -> Result<()> {
        self.state.var_domain.update(var);
        let Some(gate_poly) = self.ctx.poly_map.get(&var) else { return Ok(()); };
        debug!("Reduce var: {:?}, size: {:?}", var, gate_poly.size());

        let new_vars = gate_poly.vars();
        let gate_poly = if let Some(flip_manager) = &self.state.flip_manager {
            flip_manager.normalize(&var, &new_vars, gate_poly)
        } else {
            gate_poly.clone()
        };

        if let Some(guard) = guard {
            self.state.poly.substitute_by_poly_checked(&var, &gate_poly, guard)?;
        } else {
            self.state.poly.substitute_by_poly(&var, &gate_poly);
        }
        self.normalize();
        
        if let Some(flip_manager) = &mut self.state.flip_manager {
            flip_manager.greedy_flip(&mut self.state.poly, &new_vars);
        }
        Ok(())
    }

    pub fn run(mut self, policy: &mut dyn ReductionPolicy) -> Result<ReductionState> {
        let mut curr = 0;
        loop {
            match policy.next_action(&mut self) {
                ReductionAction::Stop => break,
                ReductionAction::Reduce(var) => {
                    self.reduce_var(var, None)?;
                }
                ReductionAction::Replace(state) => {
                    self.state = state;
                }
                ReductionAction::Skip => {}
            }
            curr += 1;
            debug!("Size: {:?}, {:?}/{:?}", self.state.poly.size(), curr, self.state.var_domain.len());

            if let Some(stats) = self.stats.as_mut() {
                stats.update_size(self.state.poly.size());
            }            
            if let Some(size_limit) = self.size_limit {
                if self.state.poly.size() > size_limit {
                    return Err(format!("Size limit exceeded: {:?}", self.state.poly.size()).into());
                }
            }
        }
        Ok(self.state)
    }
}    
