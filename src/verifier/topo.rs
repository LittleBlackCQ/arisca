use crate::bipoly::{VarId};

use std::collections::{HashMap};

pub trait VarDomain {
    fn candidates(&self) -> &[VarId];
    fn update(&mut self, target: VarId);
}
pub struct TopoVar {
    adj: HashMap<VarId, Vec<VarId>>,
    in_degree: HashMap<VarId, usize>,
    queue: Vec<VarId>,
}

impl TopoVar { 
    pub fn new(adj: HashMap<VarId, Vec<VarId>>) -> Self {
        let mut in_degree = HashMap::new();
        for (v, adj) in &adj {
            in_degree.entry(*v).or_insert(0);
            for u in adj {
                *in_degree.entry(*u).or_insert(0) += 1;
            }
        }
        let queue = in_degree.iter().filter_map(|(v, &in_degree)| if in_degree == 0 { Some(*v) } else { None }).collect();
        TopoVar { adj, in_degree, queue }
    }
}
impl VarDomain for TopoVar {
    fn candidates(&self) -> &[VarId] {
        &self.queue
    }

    fn update(&mut self, target: VarId) {
        if let Some(pos) = self.queue.iter().position(|&x| x == target) {
            self.queue.swap_remove(pos);
        }
        if let Some(neighbors) = self.adj.get(&target) {
            for neighbor in neighbors {
                if let Some(deg) = self.in_degree.get_mut(neighbor) {
                    *deg -= 1;
                    if *deg == 0 {
                        self.queue.push(*neighbor);
                    }
                }
            }
        }
    }
}

pub struct VecVar {
    candidates: Vec<VarId>,
}

impl VecVar {
    pub fn new(candidates: Vec<VarId>) -> Self {
        VecVar { candidates }
    }
}

impl VarDomain for VecVar {
    fn candidates(&self) -> &[VarId] {
        &self.candidates[self.candidates.len().saturating_sub(1)..]
    }

    fn update(&mut self, _target: VarId) {
        self.candidates.pop();
    }
}
