use super::mono::{Monomial, VarId};
use super::spec::CircuitSpec;
use super::strategy::Strategy;
use crate::circuit::*;

use num_bigint::BigInt;
use num_traits::{Zero, One, Signed};
use std::collections::HashMap;
use std::time::Instant;
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

    pub fn divide_by_term(&self, t: &[VarId]) -> Self {
        let mut res = Polynomial::zero();
        'outer: for m in self.terms.iter() {
            let mut new_m = m.clone();
            for v in t {
                if !new_m.remove_var(v) {
                    continue 'outer;
                }
            }
            res.insert(new_m);
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
        let poly = self.divide_by_term(&[*var]);
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
            let r = c % &n;
            *m = Monomial::new(m.term(), r);
        }

        // Use is_zero() for BigInt check
        self.terms.retain(|m| !m.coeff().is_zero());
    }
}


pub struct AlgebraicCircuit<'a> {
    pub inner: &'a Circuit,
    net_to_var: Vec<VarId>
}

impl<'a> AlgebraicCircuit<'a> { 
    pub fn new(circuit: &'a Circuit) -> Self {
        let net_to_var = (0..circuit.nets().len() as u32).collect();
        Self {
            inner: circuit,
            net_to_var
        }
    }

    pub fn var(&self, net: NetId) -> VarId {
        self.net_to_var[net]
    }
}
pub struct PolyVerifier;
impl PolyVerifier {
    fn init_poly_map(ac: &AlgebraicCircuit) -> HashMap<NetId, Polynomial> {
        let mut poly_map = HashMap::new();

        let one = BigInt::one();
        let two = BigInt::from(2);
        let four = BigInt::from(4);

        for node in ac.inner.nodes() {
            let inputs: Vec<VarId> = node.inputs().iter().map(|lit| ac.var(lit.net())).collect();
            let outputs: Vec<VarId> = node.outputs().iter().map(|net| ac.var(*net)).collect();
            
            let mut res = vec![Polynomial::zero(); node.gate().n_outputs()];

            match node.gate() {
                Gate::And => { 
                    res[0] += Polynomial::var(outputs[0], one.clone()) - Polynomial::term(&[inputs[0], inputs[1]], one.clone());
                }
                Gate::Or => { 
                    res[0] += Polynomial::var(outputs[0], one.clone()) -
                            (Polynomial::var(inputs[0], one.clone())
                            + Polynomial::var(inputs[1], one.clone())
                            - Polynomial::term(&[inputs[0], inputs[1]], one.clone()));
                }
                Gate::Xor => { 
                    res[0] += Polynomial::var(outputs[0], one.clone()) -
                            (Polynomial::var(inputs[0], one.clone())
                            + Polynomial::var(inputs[1], one.clone())
                            - Polynomial::term(&[inputs[0], inputs[1]], two.clone()));
                }
                Gate::Xor3 => { 
                    let sum_linear = Polynomial::var(inputs[0], one.clone()) + Polynomial::var(inputs[1], one.clone()) + Polynomial::var(inputs[2], one.clone());
                    let sum_quad = Polynomial::term(&[inputs[0], inputs[1]], two.clone()) + Polynomial::term(&[inputs[1], inputs[2]], two.clone()) + Polynomial::term(&[inputs[0], inputs[2]], two.clone());
                    let cubic = Polynomial::term(&[inputs[0], inputs[1], inputs[2]], four.clone());
                    
                    res[0] += Polynomial::var(outputs[0], one.clone()) - (sum_linear - sum_quad + cubic);
                }
                Gate::Maj => { 
                    let sum_quad = Polynomial::term(&[inputs[0], inputs[1]], one.clone()) + Polynomial::term(&[inputs[1], inputs[2]], one.clone()) + Polynomial::term(&[inputs[0], inputs[2]], one.clone());
                    let cubic = Polynomial::term(&[inputs[0], inputs[1], inputs[2]], two.clone());
                    res[0] += Polynomial::var(outputs[0], one.clone()) - (sum_quad - cubic);
                }
                Gate::HalfAdder => {
                    res[0] += Polynomial::var(outputs[0], one.clone()) - Polynomial::term(&[inputs[0], inputs[1]], one.clone());
                    res[1] += Polynomial::var(outputs[1], one.clone()) + Polynomial::var(outputs[0], two.clone()) -
                            (Polynomial::var(inputs[0], one.clone())
                            + Polynomial::var(inputs[1], one.clone()));
                }
                Gate::FullAdder => {
                    res[0] += Polynomial::var(outputs[0], one.clone()) -
                            (Polynomial::term(&[inputs[0], inputs[1]], one.clone())
                            + Polynomial::term(&[inputs[0], inputs[2]], one.clone())
                            + Polynomial::term(&[inputs[1], inputs[2]], one.clone())
                            - Polynomial::term(&[inputs[0], inputs[1], inputs[2]], two.clone()));
                    res[1] += Polynomial::var(outputs[1], one.clone()) + Polynomial::var(outputs[0], two.clone()) -
                            (Polynomial::var(inputs[0], one.clone())
                            + Polynomial::var(inputs[1], one.clone())
                            + Polynomial::var(inputs[2], one.clone()));
                }
            }

            for p in res.iter_mut() {
                for lit in node.inputs().iter() {
                    if lit.negative() {
                        p.neg_var(&ac.var(lit.net()));
                    }
                }
            }

            for (i, &net) in node.outputs().iter().enumerate() {
                poly_map.insert(net, res[i].clone());
            }
        }

        poly_map
    }

    pub fn verify<S: CircuitSpec, O: Strategy>(
        circuit: &Circuit, 
        spec: S, 
        strategy: O
    ) -> bool {
        let start_time = Instant::now();

        let ac = AlgebraicCircuit::new(circuit);

        let mut poly_map = Self::init_poly_map(&ac);

        strategy.apply(&ac, &mut poly_map);

        let mut golden_poly = spec.build_golden(&ac);
        let modulus = spec.modulus(&ac);
        
        debug!("Golden polynomial terms: {:?}\n{:?}", golden_poly.terms().len(), golden_poly);
        for net in (0..circuit.nets().len()).rev() {
            if let Some(gate_poly) = poly_map.get(&net) {
                let gate_poly = gate_poly.clone();
                debug!("Polynomial for Net {:?}: {:?}", net, gate_poly);

                if let Some(mono) = gate_poly.leading_term() {
                    if mono.degree() != 1 || !mono.coeff().is_one() {
                        warn!("Net {:?} polynomial has non-unit leading term: {:?}", net, mono);
                    }
                    
                    let factor = golden_poly.divide_by_term(mono.term());
                    
                    if !factor.is_zero() {
                        golden_poly -= gate_poly * factor;
                    } else { continue; }
                }
                if let Some(m) = &modulus {
                    golden_poly.mod_by_const(m);
                }
                debug!("Terms after reducing: {:?}", golden_poly.terms().len());
            }
        }

        let success = golden_poly.is_zero();
        info!("Verification {} in {:?}", success, start_time.elapsed());
        success
    }
}
