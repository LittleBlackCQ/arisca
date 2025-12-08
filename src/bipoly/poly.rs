use std::collections::{HashMap, hash_map::Entry};
use crate::bipoly::mono::Monomial;

#[derive(Clone)]
pub struct Polynomial {
    terms: HashMap<Monomial, i64>
}

impl Polynomial {
    pub fn zero() -> Self {
        Polynomial {
            terms: HashMap::new()
        }
    }

    pub fn constant(c: i64) -> Self {
        Polynomial {
            terms: HashMap::from([(Monomial::one(), c)])
        }
    }

    pub fn insert(&mut self, m: Monomial, c: i64) {
        if c == 0 {
            return;
        }
        match self.terms.entry(m) {
            Entry::Vacant(e) => {
                e.insert(c);
            }
            Entry::Occupied(mut e) => {
                let v = e.get_mut();
                *v += c;
                if *v == 0 {
                    e.remove();
                }
            }
        }
    }

    pub fn terms(&self) -> &HashMap<Monomial, i64> {
            &self.terms
        }

    pub fn add_assign(&mut self, rhs: &Polynomial) {
        for (m, c) in rhs.terms.iter() {
            self.insert(m.clone(), *c);
        }
    }

    pub fn add(&self, rhs: &Polynomial) -> Polynomial {
        let mut res = self.clone();
        res.add_assign(rhs);
        res
    }

    pub fn mul_assign(&mut self, rhs: &Polynomial) { 
        let mut res = Polynomial::zero();
        res.terms.reserve(self.terms.len() * rhs.terms.len());
        for (m1, c1) in self.terms.iter() {
            for (m2, c2) in rhs.terms.iter() {
                let m = m1.mul(&m2);
                res.insert(m, c1 * c2);
            }
        }
        *self = res;
    }

    pub fn mul(&self, rhs: &Polynomial) -> Polynomial {
        let mut res = self.clone();
        res.mul_assign(rhs);
        res
    }
}


