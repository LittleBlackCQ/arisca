use super::mono::{Monomial, VarId};
use super::spec::CircuitSpec;
use crate::circuit::*;

use num_bigint::BigInt;
use num_traits::{Zero, One, Signed}; // Signed trait needed for abs()
use log::{info, debug, warn};

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

    pub fn constant(c: BigInt) -> Self {
        if c.is_zero() {
            Polynomial::zero()
        } else {
            Polynomial { terms: vec![Monomial::constant(c)] }
        }
    }

    pub fn var(v: VarId, c: BigInt) -> Self {
        Polynomial { terms: vec![Monomial::new(&[v], c)] }
    }

    pub fn term(term: &[VarId], c: BigInt) -> Self {
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
            res.insert(Monomial::new(&new_vars, m.coeff().clone()));
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
                // add_coeff now expects &BigInt
                self.terms[idx].add_coeff(m.coeff());
                if self.terms[idx].coeff().is_zero() {
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

    pub fn mod_by_const(&mut self, n: &BigInt) {
        let n = n.abs();
        
        for m in self.terms.iter_mut() {
            let c = m.coeff(); // &BigInt
            // Euclidean modulo for BigInt: ((c % n) + n) % n
            let r = ((c % &n) + &n) % &n;
            // Assuming we need to replace the monomial or update coefficient
            // Since Monomial::new takes ownership, and we are mutating in place:
            *m = Monomial::new(m.term(), r);
        }

        // Use is_zero() for BigInt check
        self.terms.retain(|m| !m.coeff().is_zero());
    }
}

pub trait NodePoly {
    fn poly_eval<F>(&self, map: F) -> Vec<Polynomial> 
    where F: Fn(NetId) -> VarId;
}

impl NodePoly for Node {
    fn poly_eval<F>(&self, map: F) -> Vec<Polynomial> 
    where F: Fn(NetId) -> VarId {
        let inputs: Vec<VarId> = self.inputs().iter().map(|lit| map(lit.net())).collect();
        let outputs: Vec<VarId> = self.outputs().iter().map(|net| map(*net)).collect();
        
        let one = BigInt::one();
        let two = BigInt::from(2);
        let four = BigInt::from(4);

        let mut res = vec![Polynomial::zero(); self.gate().n_outputs()];

        match self.gate() {
            Gate::And => { // x=ab
                res[0] += Polynomial::var(outputs[0], one.clone()) - Polynomial::term(&[inputs[0], inputs[1]], one.clone());
            }
            Gate::Or => { // x=a+b-ab
                res[0] += Polynomial::var(outputs[0], one.clone()) -
                         (Polynomial::var(inputs[0], one.clone())
                        + Polynomial::var(inputs[1], one.clone())
                        - Polynomial::term(&[inputs[0], inputs[1]], one.clone()));
            }
            Gate::Xor => { // x=a+b-2ab
                res[0] += Polynomial::var(outputs[0], one.clone()) -
                         (Polynomial::var(inputs[0], one.clone())
                        + Polynomial::var(inputs[1], one.clone())
                        - Polynomial::term(&[inputs[0], inputs[1]], two.clone()));
            }
            Gate::Xor3 => { // x=a+b+c-2(ab+bc+ca)+4abc
                let sum_linear = Polynomial::var(inputs[0], one.clone()) + Polynomial::var(inputs[1], one.clone()) + Polynomial::var(inputs[2], one.clone());
                let sum_quad = Polynomial::term(&[inputs[0], inputs[1]], two.clone()) + Polynomial::term(&[inputs[1], inputs[2]], two.clone()) + Polynomial::term(&[inputs[0], inputs[2]], two.clone());
                let cubic = Polynomial::term(&[inputs[0], inputs[1], inputs[2]], four.clone());
                
                res[0] += Polynomial::var(outputs[0], one.clone()) - (sum_linear - sum_quad + cubic);
            }
            Gate::Maj => { // x=ab+bc+ca-2abc
                let sum_quad = Polynomial::term(&[inputs[0], inputs[1]], one.clone()) + Polynomial::term(&[inputs[1], inputs[2]], one.clone()) + Polynomial::term(&[inputs[0], inputs[2]], one.clone());
                let cubic = Polynomial::term(&[inputs[0], inputs[1], inputs[2]], two.clone());
                res[0] += Polynomial::var(outputs[0], one.clone()) - (sum_quad - cubic);
            }
            Gate::HalfAdder => {
                // s+2c=a+b
                res[0] += Polynomial::var(outputs[0], one.clone()) + Polynomial::var(outputs[1], two.clone()) -
                         (Polynomial::var(inputs[0], one.clone())
                        + Polynomial::var(inputs[1], one.clone()));
                // c = ab
                res[1] += Polynomial::var(outputs[1], one.clone()) - Polynomial::term(&[inputs[0], inputs[1]], one.clone());
            }
            Gate::FullAdder => {
                // s+2c = a+b+c
                res[0] += Polynomial::var(outputs[0], one.clone()) + Polynomial::var(outputs[1], two.clone()) -
                         (Polynomial::var(inputs[0], one.clone())
                        + Polynomial::var(inputs[1], one.clone())
                        + Polynomial::var(inputs[2], one.clone()));
                // c = ab+ac+bc-2abc
                res[1] += Polynomial::var(outputs[1], one.clone()) -
                         (Polynomial::term(&[inputs[0], inputs[1]], one.clone())
                        + Polynomial::term(&[inputs[0], inputs[2]], one.clone())
                        + Polynomial::term(&[inputs[1], inputs[2]], one.clone())
                        - Polynomial::term(&[inputs[0], inputs[1], inputs[2]], two.clone()));
            }
        }

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

pub struct PolyVerifier;

impl PolyVerifier {
    pub fn verify<S: CircuitSpec>(circuit: &Circuit, spec: S) -> bool {
        let mut golden_poly = spec.build_golden(circuit);
        let modulus = spec.modulus(circuit);
        debug!("Golden polynomial terms: {:?}\n{:?}", golden_poly.terms().len(), golden_poly);
        for &net in circuit.topology_order().iter().rev() {
            if let Some(driver) = circuit.nets()[net].driver() {
                let node = &circuit.nodes()[driver];

                let gate_polys = node.poly_eval(|n| circuit.get_topo_index(n) as u32);
                let output_idx = node.outputs().iter().position(|&n| n == net).unwrap();
                let gate_poly = gate_polys[output_idx].clone();
                debug!("Polynomial for Node {:?} Net {:?}: {:?}", driver, net, gate_poly);

                if let Some(term) = gate_poly.leading_term() {
                    // Check degree and coefficient (must be 1)
                    if term.degree() != 1 || !term.coeff().is_one() {
                        warn!("Node {:?} (Net {:?}) has non-unit leading term: {:?}", driver, net, term);
                    }
                    
                    let var = term.term().first().unwrap();
                    let factor = golden_poly.divide_by_var(var);
                    
                    if !factor.is_zero() {
                        golden_poly.sub_assign(&(gate_poly * factor));
                    }
                }
                if let Some(m) = &modulus {
                    golden_poly.mod_by_const(m);
                }
                debug!("Terms after reduce: {:?}", golden_poly.terms().len());
            }
        }

        let success = golden_poly.is_zero();
        info!("Verification Result: {}", success);
        success
    }
}
