use super::mono::{Monomial, VarId};

use rug::Integer;

#[derive(Clone)]
pub struct Polynomial {
    pub terms: Vec<Monomial>, // ascending order
}

impl Polynomial {
    pub fn zero() -> Self {
        Polynomial {
            terms: vec![],
        }
    }

    pub fn constant(c: Integer) -> Self {
        if c.is_zero() {
            Polynomial::zero()
        } else {
            Polynomial { terms: vec![Monomial::constant(c)] }
        }
    }

    pub fn var(v: VarId, c: Integer) -> Self {
        Polynomial { terms: vec![Monomial::new(&[v], c)] }
    }

    pub fn term(term: &[VarId], c: Integer) -> Self {
        Polynomial { terms: vec![Monomial::new(term, c)] }
    }

    pub fn mono(m: Monomial) -> Self {
        Polynomial { terms: vec![m] }
    }

    pub fn divide_by_term(&self, t: &[VarId]) -> Self {
        let mut res = Polynomial::zero();

        'outer: for m in &self.terms {
            let mut i = 0; // index in m.term
            let mut j = 0; // index in t
            let mut new_term = Vec::with_capacity(m.size());

            while i < m.size() && j < t.len() {
                if m.term()[i] < t[j] {
                    new_term.push(m.term()[i]);
                    i += 1;
                } else if m.term()[i] == t[j] {
                    i += 1;
                    j += 1;
                } else {
                    continue 'outer;
                }
            }

            if j < t.len() {
                continue;
            }

            new_term.extend_from_slice(&m.term()[i..]);

            res.terms.push(Monomial { term: new_term, coeff: m.coeff().clone() });
        }

        res
    }

    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    pub fn terms(&self) -> &[Monomial] {
        &self.terms
    }

    pub fn size(&self) -> usize {
        self.terms.len()
    }
    
    pub fn insert(&mut self, m: Monomial) {
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
        self.add_assign(poly);
    }

    pub fn add_assign(&mut self, rhs: Polynomial) {
        let lhs_terms = std::mem::take(&mut self.terms);
        let rhs_terms = rhs.terms;

        let mut new_terms = Vec::with_capacity(lhs_terms.len() + rhs_terms.len());

        let mut lhs_iter = lhs_terms.into_iter().peekable();
        let mut rhs_iter = rhs_terms.into_iter().peekable();

        loop {
            let which = match (lhs_iter.peek(), rhs_iter.peek()) {
                (Some(l), Some(r)) => Some(l.cmp(r)),
                (Some(_), None) => Some(std::cmp::Ordering::Less),
                (None, Some(_)) => Some(std::cmp::Ordering::Greater),
                (None, None) => None,
            };

            match which {
                Some(std::cmp::Ordering::Less) => {
                    new_terms.push(lhs_iter.next().unwrap());
                }
                Some(std::cmp::Ordering::Greater) => {
                    new_terms.push(rhs_iter.next().unwrap());
                }
                Some(std::cmp::Ordering::Equal) => {
                    let mut l_term = lhs_iter.next().unwrap();
                    let r_term = rhs_iter.next().unwrap();

                    l_term.add_coeff(r_term.coeff());

                    if !l_term.coeff().is_zero() {
                        new_terms.push(l_term);
                    }
                }
                None => break,
            }
        }

        self.terms = new_terms;
    }

    pub fn sub_assign(&mut self, rhs: Polynomial) {
        let lhs_terms = std::mem::take(&mut self.terms);
        let rhs_terms = rhs.terms; // Owned

        let mut new_terms = Vec::with_capacity(lhs_terms.len() + rhs_terms.len());
        
        let mut lhs_iter = lhs_terms.into_iter().peekable();
        let mut rhs_iter = rhs_terms.into_iter().peekable();

        loop {
            let which = match (lhs_iter.peek(), rhs_iter.peek()) {
                (Some(l), Some(r)) => Some(l.cmp(r)),
                (Some(_), None) => Some(std::cmp::Ordering::Less),
                (None, Some(_)) => Some(std::cmp::Ordering::Greater),
                (None, None) => None,
            };

            match which {
                Some(std::cmp::Ordering::Less) => {
                    new_terms.push(lhs_iter.next().unwrap());
                }
                Some(std::cmp::Ordering::Greater) => {
                    let mut term = rhs_iter.next().unwrap();
                    term.neg_coeff(); 
                    new_terms.push(term);
                }
                Some(std::cmp::Ordering::Equal) => {
                    let mut l_term = lhs_iter.next().unwrap();
                    let mut r_term = rhs_iter.next().unwrap();

                    r_term.neg_coeff();
                    l_term.add_coeff(r_term.coeff());

                    if !l_term.coeff().is_zero() {
                        new_terms.push(l_term);
                    }
                }
                None => break,
            }
        }

        self.terms = new_terms;
    }

    pub fn neg_assign(&mut self) {
        for m in self.terms.iter_mut() {
            m.neg_coeff();
        }
    }

    pub fn mul_assign(&mut self, rhs: Polynomial) {
        if self.terms.is_empty() || rhs.terms.is_empty() {
            self.terms.clear();
            return;
        }

        let mut raw_terms = Vec::with_capacity(self.terms.len() * rhs.terms.len());
        
        for m1 in self.terms.iter() {
            for m2 in rhs.terms.iter() {
                raw_terms.push(m1.mul(m2));
            }
        }

        raw_terms.sort_unstable();

        let mut dedup_terms = Vec::with_capacity(raw_terms.len());
        let mut iter = raw_terms.into_iter();

        if let Some(mut current_term) = iter.next() {
            for next_term in iter {
                if next_term == current_term {
                    current_term.add_coeff(next_term.coeff());
                } else {
                    if !current_term.coeff().is_zero() {
                        dedup_terms.push(current_term);
                    }
                    current_term = next_term;
                }
            }
            if !current_term.coeff().is_zero() {
                dedup_terms.push(current_term);
            }
        }

        self.terms = dedup_terms;
    }

    pub fn mod_by_const(&mut self, n: &Integer) {
        for m in self.terms.iter_mut() {
            m.coeff %= n;
        }

        // Use is_zero() for BigInt check
        self.terms.retain(|m| !m.coeff().is_zero());
    }

    pub fn remove_mono_by<F>(&mut self, f: F)
    where F: Fn(&Monomial) -> bool {
        self.terms.retain(|m| !f(m));
    }
}
