mod term;
mod ops;
mod debug;

pub use term::{VarId, Term};
use rug::Integer;
use std::collections::{BTreeMap, btree_map::Entry, HashSet};

#[derive(Clone, Default)]
pub struct Polynomial {
    pub terms: BTreeMap<Term, Integer>,
}

impl Polynomial {
    pub fn new() -> Self {
        Self {
            terms: BTreeMap::new(),
        }
    }

    pub fn from_constant(c: Integer) -> Self {
        let mut terms = BTreeMap::new();
        if !c.is_zero() {
            terms.insert(Term { vars: vec![] }, c);
        }
        Self { terms }
    }

    pub fn from_var(v: VarId, c: Integer) -> Self {
        let mut terms = BTreeMap::new();
        if !c.is_zero() {
            terms.insert(Term { vars: vec![v] }, c);
        }
        Self { terms }
    }

    pub fn from_vars(vars: Vec<i32>, c: Integer) -> Self {
        let mut terms = BTreeMap::new();
        if !c.is_zero() {
            terms.insert(Term::new(vars), c);
        }
        Self { terms }
    }

    pub fn from_term(t: Term, c: Integer) -> Self {
        let mut terms = BTreeMap::new();
        if !c.is_zero() {
            terms.insert(t, c);
        }
        Self { terms }
    }

    pub fn size(&self) -> usize {
        self.terms.len()
    }

    pub fn terms(&self) -> &BTreeMap<Term, Integer> {
        &self.terms
    }

    pub fn vars(&self) -> Vec<VarId> {
        self.terms.keys().flat_map(|t| t.vars.clone()).collect::<HashSet<_>>().into_iter().collect()
    }

    pub fn coeff_of(&self, term: &Term) -> Integer {
        match self.terms.get(term) {
            Some(c) => c.clone(),
            None => Integer::from(0),
        }
    }

    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    pub fn insert(&mut self, term: Term, coeff: Integer) {
        if coeff.is_zero() { return; }

        match self.terms.entry(term) {
            Entry::Occupied(mut entry) => {
                let c = entry.get_mut();
                *c += coeff;
                if c.is_zero() {
                    entry.remove();
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(coeff);
            }
        }
    }

    pub fn add_assign(&mut self, rhs: Polynomial) {
        for (term, coeff) in rhs.terms {
            self.insert(term, coeff);
        }
    }

    pub fn sub_assign(&mut self, rhs: Polynomial) {
        for (term, coeff) in rhs.terms {
            self.insert(term, -coeff);
        }
    }

    pub fn mul_assign(&mut self, rhs: &Polynomial) {
        if self.is_zero() || rhs.is_zero() {
            self.terms.clear();
            return;
        }

        let mut next_map = Polynomial::new();
        let lhs_map = std::mem::take(&mut self.terms);

        for (t1, c1) in lhs_map.iter() {
            for (t2, c2) in rhs.terms.iter() {
                let merged_term = t1.mul(t2);
                let prod = Integer::from(c1 * c2);
                next_map.insert(merged_term, prod);
            }
        }
        self.terms = next_map.terms;
    }

    pub fn divide_by_var(&self, v: &VarId) -> Self {
        let t = Term { vars: vec![v.clone()] };
        self.divide_by_term(&t)
    }
    
    pub fn divide_by_term(&self, t: &Term) -> Self {
        if t.size() == 0 {
            return self.clone();
        }
        let mut res = Polynomial::new();

        for (m_term, m_coeff) in &self.terms {
            if let Some(new_term) = m_term.div(t) {
                res.terms.insert(new_term, m_coeff.clone());
            }
        }
        res
    }

    pub fn substitute_by_poly(&mut self, v: &VarId, poly: &Polynomial) {
        let mut new_poly = Polynomial::new();
        self.terms.retain(|term, coeff| {
            if let Some(new_term) = term.remove_var(v) {
                new_poly += Polynomial::from_term(new_term, coeff.clone()) * poly;
                false
            } else {
                true
            }
        });
        *self += new_poly;
    }

    pub fn neg_var(&mut self, v: &VarId) {
        let mut extracted_poly = Self::new();

        for (term, coeff) in self.terms.iter_mut() {
            if let Some(new_term) = term.remove_var(v) {
                extracted_poly.insert(new_term, coeff.clone());
                *coeff = -std::mem::take(coeff);
            }
        }
        *self += extracted_poly;
    }

    pub fn neg_coeff(&mut self) {
        for (_, coeff) in self.terms.iter_mut() {
            *coeff = -std::mem::take(coeff);
        }
    }

    pub fn neg_self(&mut self) {
        *self -= Polynomial::from_constant(Integer::from(1));
        self.neg_coeff();
    }

    pub fn mod_by_const(&mut self, n: &Integer) {
        self.terms.retain(|_, coeff| {
            *coeff %= n;
            !coeff.is_zero()
        });
    }

    pub fn remove_mono_by<F>(&mut self, f: F)
    where 
        F: Fn(&Term, &Integer) -> bool 
    {
        self.terms.retain(|term, coeff| !f(term, coeff));
    }
}

#[cfg(test)]
mod tests { 
    use super::*;

    #[test]
    fn test_add() { 
        let mut a = Polynomial::new();
        let t1 = Term::new(vec![0]);
        let t2 = Term::new(vec![1, 2]);
        a.insert(t1.clone(), Integer::from(1));
        a.insert(t2.clone(), Integer::from(2));

        let mut b = Polynomial::new();
        b.insert(t1.clone(), Integer::from(-1));
        b.insert(t2.clone(), Integer::from(-3));

        let res = a + b;
        assert_eq!(res.terms.len(), 1);
        assert_eq!(res.coeff_of(&t1), Integer::from(0));
        assert_eq!(format!("{:?}", res), "-1x1*x2");
    }

    #[test]
    fn test_mul() {
        let t1 = Term::new(vec![0]);
        let t2 = Term::new(vec![1]);
        let t3 = Term::new(vec![2]);

        let mut a = Polynomial::new();
        a.insert(t1.clone(), Integer::from(1));
        a.insert(t2.clone(), Integer::from(2));

        let mut b = Polynomial::new();
        b.insert(t1.clone(), Integer::from(2));
        b.insert(t3.clone(), Integer::from(-1));

        let res = a * b;
        assert_eq!(res.terms.len(), 4);
        assert_eq!(format!("{:?}", res), "2x0+4x0*x1-1x0*x2-2x1*x2");
    }

    #[test]
    fn test_neg_var() { 
        let a = Polynomial::from_var(0, Integer::from(1));
        let b = Polynomial::from_var(1, Integer::from(1));
        let one = Polynomial::from_constant(Integer::from(1));

        let mut c = (&one - a) * (&one - b);

        c.neg_var(&0);
        c.neg_var(&1);
        assert_eq!(format!("{:?}", c), "1x0*x1");
    }
}