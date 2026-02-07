use rug::Integer;

pub type VarId = i32;
pub type Term = Vec<VarId>;

#[derive(Clone, Hash)]
pub struct Monomial {
    pub(super) coeff: Integer,
    pub(super) term: Term,
}

impl Monomial {
    pub fn new(vars: &[VarId], coeff: Integer) -> Self {
        let mut term = vars.to_vec();
        term.sort_unstable();
        term.dedup();
        Monomial { coeff, term }
    }

    pub fn constant(coeff: Integer) -> Self {
        assert!(!coeff.is_zero());
        Monomial::new(&[], coeff)
    }

    pub fn vars(v: &[VarId]) -> Self {
        Monomial::new(v, Integer::from(1))
    }

    pub fn coeff(&self) -> &Integer {
        &self.coeff
    }

    pub fn term(&self) -> &[VarId] {
        &self.term
    }

    pub fn size(&self) -> usize {
        self.term.len()
    }

    pub fn contains(&self, v: &VarId) -> bool {
        self.term.binary_search(v).is_ok()
    }

    // Accept reference to avoid moving/cloning rhs
    pub fn add_coeff(&mut self, rhs: &Integer) {
        self.coeff += rhs;
    }

    pub fn neg_coeff(&mut self) {
        self.coeff = Integer::from(-&self.coeff);
    }

    pub fn remove_var(&mut self, v: &VarId) -> bool {
        if let Ok(pos) = self.term.binary_search(v) {
            self.term.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn mul(&self, rhs: &Monomial) -> Self {
        let new_coeff = Integer::from(&self.coeff * &rhs.coeff);

        let va = &self.term;
        let vb = &rhs.term;
        let mut new_term = Vec::with_capacity(va.len() + vb.len());

        let mut pi = 0;
        let mut qi = 0;

        while pi < va.len() && qi < vb.len() {
            if va[pi] < vb[qi] {
                new_term.push(va[pi].clone());
                pi += 1;
            } else if va[pi] > vb[qi] {
                new_term.push(vb[qi].clone());
                qi += 1;
            } else {
                new_term.push(va[pi].clone());
                pi += 1;
                qi += 1;
            }
        }

        if pi < va.len() {
            new_term.extend_from_slice(&va[pi..]);
        } else {
            new_term.extend_from_slice(&vb[qi..]);
        }

        Monomial {
            coeff: new_coeff,
            term: new_term,
        }
    }
}

impl PartialEq for Monomial {
    fn eq(&self, other: &Self) -> bool {
        self.term == other.term
    }
}

impl Eq for Monomial {}

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
