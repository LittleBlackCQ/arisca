use crate::bipoly::mono::{Monomial, VarId};
use crate::circuit::{Circuit, gate::Gate, basics::{Node, NetId}};

use log::{debug, warn};

#[derive(Clone)]
pub struct Polynomial {
    terms: Vec<Monomial>, // ascending order
}

impl Polynomial {
    pub fn zero() -> Self {
        Polynomial {
            terms: vec![],
        }
    }

    pub fn constant(c: i64) -> Self {
        if c == 0 {
            Polynomial::zero()
        } else {
            Polynomial { terms: vec![Monomial::constant(c)] }
        }
    }

    pub fn var(v: VarId, c: i64) -> Self {
        Polynomial { terms: vec![Monomial::new(&[v], c)] }
    }

    pub fn term(term: &[VarId], c: i64) -> Self {
        Polynomial { terms: vec![Monomial::new(term, c)] }
    }

    pub fn mono(m: Monomial) -> Self {
        Polynomial { terms: vec![m] }
    }
    
    pub fn remap<F>(&self, map: F) -> Self 
    where F: Fn(VarId) -> VarId {
        let mut res = Polynomial::zero();
        for m in self.terms.iter() {
            let new_vars: Vec<VarId> = m.term().iter().map(|&v| map(v)).collect();
            res.insert(Monomial::new(&new_vars, m.coeff()));
        }
        res
    }

    pub fn divide_by_var(&self, v: &VarId) -> Self {
        let mut res = Polynomial::zero();
        for m in self.terms.iter() {
            let mut new_m = m.clone();
            if new_m.remove_var(v) {
                res.insert(new_m);
            } 
        }
        res
    }

    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    pub fn terms(&self) -> &[Monomial] {
            &self.terms
        }

    pub fn leading_term(&self) -> Option<&Monomial> {
            if self.terms.is_empty() {
                return None;
            }
            Some(&self.terms[self.terms.len()-1])
        }
    
    fn insert(&mut self, m: Monomial) {
        let pos = self.terms.binary_search(&m);
        match pos {
            Ok(idx) => {
                self.terms[idx].add_coeff(m.coeff());
                if self.terms[idx].coeff() == 0 {
                    self.terms.remove(idx);
                }
            }
            Err(idx) => {
                self.terms.insert(idx, m);
            }

        }
    }

    pub fn neg_var(&mut self, var: &VarId) {
        let poly = self.divide_by_var(var);
        for m in self.terms.iter_mut() {
            if m.contains(var) {
                m.neg_coeff();
            }
        }
        self.add_assign(&poly);
    }

    pub fn add_assign(&mut self, rhs: &Polynomial) {
        for m in rhs.terms().iter() {
            self.insert(m.clone());
        }
    }

    pub fn sub_assign(&mut self, rhs: &Polynomial) {
        for m in rhs.terms.iter() {
            self.insert(m.neg());
        }
    }

    pub fn neg_assign(&mut self) {
        for m in self.terms.iter_mut() {
            m.neg_coeff();
        }
    }

    pub fn mul_assign(&mut self, rhs: &Polynomial) { 
        let mut res = Polynomial::zero();
        res.terms.reserve(self.terms.len() * rhs.terms.len());
        for m1 in self.terms.iter() {
            for m2 in rhs.terms.iter() {
                res.insert(m1.mul(&m2));
            }
        }
        *self = res;
    }
}

impl Gate {
    pub fn polynomial(&self, inputs: &[VarId], outputs: &[VarId]) -> Vec<Polynomial> {
        assert_eq!(inputs.len(), self.n_inputs(), "Wrong number of inputs for {} in polynomial.", self.name());
        let mut res = vec![Polynomial::zero(); self.n_outputs()];
        match self {
            Gate::And => { // x=ab
                res[0] += Polynomial::var(outputs[0], 1) - Polynomial::term(&[inputs[0], inputs[1]], 1);
            }
            Gate::Or => { // x=a+b-ab
                res[0] += Polynomial::var(outputs[0], 1) -
                         (Polynomial::var(inputs[0], 1)
                        + Polynomial::var(inputs[1], 1)
                        - Polynomial::term(&[inputs[0], inputs[1]], 1));
            }
            Gate::Xor => { // x=a+b-2ab
                res[0] += Polynomial::var(outputs[0], 1) -
                         (Polynomial::var(inputs[0], 1)
                        + Polynomial::var(inputs[1], 1)
                        - Polynomial::term(&[inputs[0], inputs[1]], 2));
            }
            Gate::Xor3 => { // x=a+b+c-2(ab+bc+ca)+4abc
                let sum_linear = Polynomial::var(inputs[0], 1) + Polynomial::var(inputs[1], 1) + Polynomial::var(inputs[2], 1);
                let sum_quad = Polynomial::term(&[inputs[0], inputs[1]], 2) + Polynomial::term(&[inputs[1], inputs[2]], 2) + Polynomial::term(&[inputs[0], inputs[2]], 2);
                let cubic = Polynomial::term(&[inputs[0], inputs[1], inputs[2]], 4);
                
                res[0] += Polynomial::var(outputs[0], 1) - (sum_linear - sum_quad + cubic);
            }
            Gate::Maj => { // x=ab+bc+ca-2abc
                let sum_quad = Polynomial::term(&[inputs[0], inputs[1]], 1) + Polynomial::term(&[inputs[1], inputs[2]], 1) + Polynomial::term(&[inputs[0], inputs[2]], 1);
                let cubic = Polynomial::term(&[inputs[0], inputs[1], inputs[2]], 2);
                res[0] += Polynomial::var(outputs[0], 1) - (sum_quad - cubic);
            }
            Gate::HalfAdder => {
                // s+2c=a+b
                res[0] += Polynomial::var(outputs[0], 1) + Polynomial::var(outputs[1], 2) -
                         (Polynomial::var(inputs[0], 1)
                        + Polynomial::var(inputs[1], 1));
                // c = ab
                res[1] += Polynomial::var(outputs[1], 1) - Polynomial::term(&[inputs[0], inputs[1]], 1);
            }
            Gate::FullAdder => {
                // s+2c = a+b+c
                res[0] += Polynomial::var(outputs[0], 1) + Polynomial::var(outputs[1], 2) -
                         (Polynomial::var(inputs[0], 1)
                        + Polynomial::var(inputs[1], 1)
                        + Polynomial::var(inputs[2], 1));
                // c = ab+ac+bc-2abc
                res[1] += Polynomial::var(outputs[1], 1) -
                         (Polynomial::term(&[inputs[0], inputs[1]], 1)
                        + Polynomial::term(&[inputs[0], inputs[2]], 1)
                        + Polynomial::term(&[inputs[1], inputs[2]], 1)
                        - Polynomial::term(&[inputs[0], inputs[1], inputs[2]], 2));
            }
        }
        assert_eq!(res.len(), self.n_outputs(), "Wrong number of outputs for {} in polynomial.", self.name());
        res
    }

}

impl Node {
    pub fn poly_eval<F>(&self, map: F) -> Vec<Polynomial> 
    where F: Fn(NetId) -> VarId {
        let inputs: Vec<VarId> = self.inputs().iter().map(|lit| map(lit.net())).collect();
        let outputs: Vec<VarId> = self.outputs().iter().map(|net| map(*net)).collect();
        
        let mut res = self.gate().polynomial(&inputs, &outputs);
        for p in res.iter_mut() {
            for lit in self.inputs().iter() {
                if lit.negative() {
                    p.neg_var(&map(lit.net()));
                }
            }
        }
        res
    }
}

impl Circuit {
    pub fn check_poly(&self, golden: &Polynomial) -> bool { 
        // 1. remap nets to topo order
        let mut net_to_topo = vec![0u32; self.nets().len()];
        for (i, &net) in self.topology_order().iter().enumerate() {
            net_to_topo[net] = i as u32;
        }
        let map_fn = |n: NetId| net_to_topo[n];

        // 2. remap golden polynomial and outputs
        let mut golden = golden.remap(|v| map_fn(v as usize));
        for output in self.outputs() {
            if output.negative() {
                golden.neg_var(&map_fn(output.net()));
            }
        }

        // 3. traverse circuit through reverse topological order
        for &net in self.topology_order().iter().rev() {
            if let Some(driver) = self.nets()[net].driver() {
                let node = self.nodes().get(driver).unwrap();
                let gate_polys = node.poly_eval(&map_fn);
                for gate_poly in gate_polys { 
                    debug!("Polynomial before reduce: {:?}", golden);
                    debug!("Polynomial for Node {:?}: {:?}",driver, gate_poly);
                    let term = gate_poly.leading_term().unwrap();
                    if term.degree() != 1 || term.coeff() != 1 {
                        warn!("Polynomial of node {:?} has non-unit leading term: {:?}", driver, term);
                    }
                    let var = term.term().first().unwrap();
                    let factor = golden.divide_by_var(var);
                    let mult = gate_poly * factor;
                    golden.sub_assign(&mult);
                    debug!("Polynomial after reduce: {:?}", golden)
                }
            }
        }
        golden.is_zero()
    }
}
