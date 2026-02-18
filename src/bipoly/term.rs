pub type VarId = i32;

#[derive(Clone)]
pub struct Term {
    pub(super) vars: Vec<VarId>,
}

impl Term {
    pub fn new(mut vars: Vec<VarId>) -> Self {
        vars.sort_unstable();
        vars.dedup();
        Term { vars }
    }

    pub fn vars(&self) -> &[VarId] {
        &self.vars
    }

    pub fn size(&self) -> usize {
        self.vars.len()
    }

    pub fn contains(&self, v: &VarId) -> bool {
        self.vars.binary_search(v).is_ok()
    }

    pub fn remove_var(&self, v: &VarId) -> Option<Term> {
        if let Ok(pos) = self.vars.binary_search(v) {
            let mut res = self.clone();
            res.vars.remove(pos);
            Some(res)
        } else {
            None
        }
    }

    pub fn mul(&self, rhs: &Term) -> Self {
        let va = self.vars();
        let vb = rhs.vars();
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

        Term {
            vars: new_term,
        }
    }


    pub fn div(&self, rhs: &Term) -> Option<Term> {
        let lhs_vars = self.vars();
        let rhs_vars = rhs.vars();
        let mut res = Vec::with_capacity(lhs_vars.len());

        let (mut i, mut j) = (0, 0);

        while i < lhs_vars.len() && j < rhs_vars.len() {
            if lhs_vars[i] < rhs_vars[j] {
                res.push(lhs_vars[i]);
                i += 1;
            } else if lhs_vars[i] == rhs_vars[j] {
                i += 1;
                j += 1;
            } else {
                return None;
            }
        }

        if j < rhs_vars.len() {
            return None;
        }

        res.extend_from_slice(&lhs_vars[i..]);
        Some(Term { vars: res })
    }

}

impl PartialEq for Term {
    fn eq(&self, other: &Self) -> bool {
        self.vars == other.vars
    }
}

impl Eq for Term {}

impl Ord for Term {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.vars.cmp(&other.vars)
    }
}

impl PartialOrd for Term {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
