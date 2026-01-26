use num_bigint::BigInt;
use num_traits::{Zero, One};

pub type VarId = i32;
pub type Term = Vec<VarId>;

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Monomial {
    coeff: BigInt,
    term: Term,
}

impl Monomial {
    pub fn new(vars: &[VarId], coeff: BigInt) -> Self {
        let mut term = vars.to_vec();
        term.sort_unstable();
        term.dedup();
        Monomial { coeff, term }
    }

    pub fn constant(coeff: BigInt) -> Self {
        assert!(!coeff.is_zero());
        Monomial::new(&[], coeff)
    }

    pub fn vars(v: &[VarId]) -> Self {
        Monomial::new(v, BigInt::one())
    }

    pub fn neg(&self) -> Self {
        Self::new(&self.term, -&self.coeff)
    }

    pub fn coeff(&self) -> &BigInt {
        &self.coeff
    }

    pub fn term(&self) -> &[VarId] {
        &self.term
    }

    pub fn degree(&self) -> usize {
        self.term.len()
    }

    pub fn contains(&self, v: &VarId) -> bool {
        self.term.binary_search(v).is_ok()
    }

    // Accept reference to avoid moving/cloning rhs
    pub fn add_coeff(&mut self, rhs: &BigInt) {
        self.coeff += rhs;
    }

    pub fn neg_coeff(&mut self) {
        self.coeff = -&self.coeff;
    }

    pub fn remove_var(&mut self, v: &VarId) -> bool {
        if let Ok(pos) = self.term.binary_search(v) {
            self.term.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn mul_assign(&mut self, rhs: &Monomial) {
        self.coeff *= &rhs.coeff;

        let va = &self.term;
        let vb = &rhs.term;

        // Merge sorted terms (linear scan)
        let mut res = Vec::with_capacity(va.len() + vb.len());
        let mut pi = 0;
        let mut qi = 0;

        while pi < va.len() && qi < vb.len() {
            if va[pi] < vb[qi] {
                res.push(va[pi]);
                pi += 1;
            } else if va[pi] > vb[qi] {
                res.push(vb[qi]);
                qi += 1;
            } else {
                res.push(va[pi]);
                pi += 1;
                qi += 1;
            }
        }

        if pi < va.len() {
            res.extend_from_slice(&va[pi..]);
        } else {
            res.extend_from_slice(&vb[qi..]);
        }

        self.term = res;
    }

    pub fn mul(&self, rhs: &Monomial) -> Monomial {
        let mut res = self.clone();
        res.mul_assign(rhs);
        res
    }
}

impl Ord for Monomial {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.term.cmp(&other.term)
    }
}

impl PartialOrd for Monomial {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
