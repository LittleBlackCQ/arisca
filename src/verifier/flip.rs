use std::collections::HashSet;
use crate::bipoly::{Polynomial, VarId};

use log::{debug};

pub struct FlipManager {
    pub visited_vars: HashSet<VarId>,
    pub flipped_vars: HashSet<VarId>,
}

impl FlipManager {
    pub fn new() -> Self {
        FlipManager {
            visited_vars: HashSet::new(),
            flipped_vars: HashSet::new(),
        }
    }

    pub fn is_flip(&self, var: &VarId) -> bool {
        self.flipped_vars.contains(var)
    }

    pub fn greedy_flip(&mut self, poly: &mut Polynomial, vars: &[VarId]) {
        for v in vars {
            if !self.visited_vars.contains(v) {
                let origin_poly = poly.clone();
                poly.neg_var(v);

                let origin_size = origin_poly.size();
                let new_size = poly.size();
                debug!("Try flip var {:?}, size: {} -> {:?}", v, origin_size, new_size);
                if new_size < origin_size {
                    self.flipped_vars.insert(*v);
                } else {
                    *poly = origin_poly;
                }
                self.visited_vars.insert(*v);
            }
        }
    }
}