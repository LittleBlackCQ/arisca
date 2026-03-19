
use super::{ReductionPolicy, ReductionContext, ReductionAction, ReductionState, ReductionStats, VarDomain, SizeGuard, process_cone, Substitution};
use crate::bipoly::{Polynomial, VarId};
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

    fn normalize(&mut self) {
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

    fn apply_reduction(&mut self, var: VarId, gate_poly: Polynomial, guard: Option<&SizeGuard>) -> Result<()> {
        debug!("Reduce var: {:?}, size: {:?}", var, gate_poly.size());
        let new_vars = gate_poly.vars();
        let gate_poly = if let Some(fm) = &self.state.flip_manager {
            fm.normalize(&var, &new_vars, &gate_poly)
        } else {
            gate_poly
        };

        if let Some(guard) = guard {
            self.state.poly.substitute_by_poly_checked(&var, &gate_poly, guard)?;
        } else {
            self.state.poly.substitute_by_poly(&var, &gate_poly);
        }

        self.normalize();
        
        if let Some(fm) = &mut self.state.flip_manager {
            fm.greedy_flip(&mut self.state.poly, &new_vars);
        }
        Ok(())
    }

    fn resolve_substitution(&mut self, var: VarId, guard: Option<&SizeGuard>) -> Result<Option<Polynomial>> {
        match self.ctx.substitutions.get(&var) {
            Some(Substitution::Poly(p)) => Ok(Some(p.clone())),
            Some(Substitution::Cone(cone, is_conv, base_poly)) => {
                self.apply_reduction(var, base_poly.clone(), guard)?;
                let new_poly = process_cone(cone, self.state.poly.clone(), *is_conv, self.ctx, None, self.state.flip_manager.as_mut())?;
                self.state.poly = new_poly;
                Ok(None)
            }
            None => Ok(None),
        }
    }

    pub fn reduce_var(&mut self, var: VarId, guard: Option<&SizeGuard>) -> Result<()> {
        self.state.var_domain.update(var);
        if let Some(gate_poly) = self.resolve_substitution(var, guard)? {
            self.apply_reduction(var, gate_poly, guard)?;
        };
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
