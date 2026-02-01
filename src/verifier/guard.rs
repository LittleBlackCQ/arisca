use crate::bipoly::{Polynomial, Monomial};

pub struct SizeGuard {
    max_terms: usize,
}

#[derive(Debug)]
pub struct SizeLimitExceeded {
    pub size: usize,
    pub limit: usize,
}

impl SizeGuard {
    pub fn new(max_terms: usize) -> Self {
        Self { max_terms }
    }

    pub fn check(&self, size: usize) -> Result<(), SizeLimitExceeded> {
        if size > self.max_terms {
            Err(SizeLimitExceeded {
                size,
                limit: self.max_terms,
            })
        } else {
            Ok(())
        }
    }
}

impl Polynomial {
    pub fn insert_checked(&mut self, m: Monomial, guard: &SizeGuard) -> Result<(), SizeLimitExceeded> {
        let before = self.size();
        self.insert(m);
        let after = self.size();
        if after > before {
            guard.check(after)?;
        }
        Ok(())
    }

    pub fn add_assign_checked(&mut self, rhs: &Polynomial, guard: &SizeGuard) -> Result<(), SizeLimitExceeded> {
        for m in rhs.terms().iter() {
            self.insert_checked(m.clone(), guard)?;
        }
        Ok(())
    }

    pub fn sub_assign_checked(&mut self, rhs: &Polynomial, guard: &SizeGuard) -> Result<(), SizeLimitExceeded> {
        for m in rhs.terms().iter() {
            self.insert_checked(m.neg(), guard)?;
        }
        Ok(())
    }

    pub fn mul_assign_checked(&mut self, rhs: &Polynomial, guard: &SizeGuard) -> Result<(), SizeLimitExceeded> { 
        let mut res = Polynomial::zero();
        for m1 in self.terms().iter() {
            for m2 in rhs.terms().iter() {
                let m = m1.mul(m2);
                res.insert_checked(m, guard)?;
            }
        }
        *self = res;
        Ok(())
    }
}
