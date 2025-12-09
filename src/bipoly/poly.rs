use crate::bipoly::mono::{Monomial, VarId};
use crate::circuit::{Circuit, gate::Gate, basics::Node};

use log::{debug};

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
        assert_eq!(inputs.len(), self.n_inputs());
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
        assert_eq!(res.len(), self.n_outputs());
        res
    }

}

impl Node {
    pub fn poly_eval(&self) -> Vec<Polynomial> {
        let inputs: Vec<u32> = self.inputs().iter().map(|lit| u32::try_from(lit.net()).expect("net index too large")).collect();
        let outputs: Vec<u32> = self.outputs().iter().map(|net| u32::try_from(*net).expect("net index too large")).collect();
        let mut res = self.gate().polynomial(&inputs, &outputs);
        for p in res.iter_mut() {
            for lit in self.inputs().iter() {
                if lit.negative() {
                    p.neg_var(&u32::try_from(lit.net()).expect("net index too large"));
                }
            }
        }
        res
    }
}

impl Circuit {
    pub fn check_poly(&self, golden: &Polynomial) -> bool { 
        let mut golden = golden.clone();
        for output in self.outputs() {
            if output.negative() {
                golden.neg_var(&u32::try_from(output.net()).expect("net index too large"));
            }
        }
        for net in self.nets().iter().rev() {
            if let Some(driver) = net.driver() {
                if let Some(node) = self.nodes().get(driver) {
                    let gate_polys = node.poly_eval();
                    for gate_poly in gate_polys { 
                        debug!("Polynomial before reduce: {:?}", golden);
                        debug!("Polynomial for Node {:?}: {:?}",driver, gate_poly);
                        if let Some(term) = gate_poly.leading_term() {
                            assert_eq!(term.degree(), 1);
                            assert_eq!(term.coeff(), 1);
                            if let Some(var) = term.term().first() {
                                let factor = golden.divide_by_var(var);
                                let mult = gate_poly * factor;
                                golden.sub_assign(&mult);
                            }
                        }
                        debug!("Polynomial after reduce: {:?}", golden)
                    }
                }
            }
        }
        golden.is_zero()
    }
}
