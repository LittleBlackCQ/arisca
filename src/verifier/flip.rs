use crate::bipoly::{Polynomial, VarId};
use std::collections::HashSet;


#[derive(Clone)]
pub struct FlipManager {
    pub flipped_vars: HashSet<VarId>,
}

impl FlipManager {
    pub fn new() -> Self {
        FlipManager {
            flipped_vars: HashSet::new(),
        }
    }

    pub fn is_flip(&self, var: &VarId) -> bool {
        self.flipped_vars.contains(var)
    }

    pub fn greedy_flip(&mut self, poly: &mut Polynomial, vars: &[VarId]) {
        for v in vars {
            if !self.flipped_vars.contains(v) && *v > 0 {
                let origin_poly = poly.clone();
                poly.neg_var(v);

                let origin_size = origin_poly.size();
                let new_size = poly.size();
                if new_size < origin_size {
                    debug_step!(
                        "FLIP"; step; "Flipped var", v;
                        "Size: {:>4} -> {:<4}", origin_size, new_size
                    );
                    self.flipped_vars.insert(*v);
                } else {
                    *poly = origin_poly;
                }
            }
        }
    }

    pub fn update(&mut self, other: &FlipManager) {
        self.flipped_vars.extend(&other.flipped_vars);
    }

    pub fn normalize(&self, root: &VarId, vars: &[VarId], poly: &Polynomial) -> Polynomial {
        let mut poly = poly.clone();
        for v in vars {
            if self.is_flip(v) {
                poly.neg_var(v);
            }
        }
        if self.is_flip(root) {
            poly.neg_self();
        }
        poly
    }
}
