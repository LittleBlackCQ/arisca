pub type VarId = u32;

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Monomial {
    vars: Vec<VarId>,
}

impl Monomial {
    pub fn new(mut vars: Vec<VarId>) -> Self {
        vars.sort_unstable();
        vars.dedup();
        Monomial { vars }
    }

    pub fn one() -> Self {
        Monomial::new(vec![])
    }

    pub fn var(v: VarId) -> Self {
        Monomial::new(vec![v])
    }

    pub fn vars(&self) -> &[VarId] {
        &self.vars
    }

    pub fn mul_assign(&mut self, rhs: &Monomial) {
        let va = &self.vars;
        let vb = &rhs.vars;

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

        self.vars = res;
    }

    pub fn mul(&self, rhs: &Monomial) -> Monomial {
        let mut res = self.clone();
        res.mul_assign(rhs);
        res
    }
    // pub fn contains(&self, v: VarId) -> bool {
    //     self.vars.contains(&v)
    // }

    // pub fn degree(&self) -> usize;

    // pub fn vars(&self) -> &[VarId];
}

