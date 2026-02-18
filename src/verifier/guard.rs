use crate::bipoly::{Polynomial, Term, VarId};
use rug::Integer;

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
    pub fn insert_checked(&mut self, term: Term, coeff: Integer, guard: &SizeGuard) -> Result<(), SizeLimitExceeded> {
        self.insert(term, coeff);
        guard.check(self.size())?;
        Ok(())
    }
    pub fn add_assign_checked(&mut self, rhs: Polynomial, guard: &SizeGuard) -> Result<(), SizeLimitExceeded> {
        for (term, coeff) in rhs.terms {
            self.insert_checked(term, coeff, guard)?
        }
        Ok(())
    }

    pub fn sub_assign_checked(&mut self, rhs: Polynomial, guard: &SizeGuard) -> Result<(), SizeLimitExceeded> {
        for (term, coeff) in rhs.terms {
            self.insert_checked(term, -coeff, guard)?;
        }
        Ok(())
    }

    pub fn mul_assign_checked(&mut self, rhs: &Polynomial, guard: &SizeGuard) -> Result<(), SizeLimitExceeded> {
        if self.is_zero() || rhs.is_zero() {
            self.terms.clear();
            return Ok(());
        }

        let mut next_map = Polynomial::new();
        let lhs_map = std::mem::take(&mut self.terms);

        for (t1, c1) in lhs_map.iter() {
            for (t2, c2) in rhs.terms.iter() {
                let merged_term = t1.mul(t2);
                let prod = Integer::from(c1 * c2);
                next_map.insert_checked(merged_term, prod, guard)?
            }
        }
        self.terms = next_map.terms;
        Ok(())
    }

    pub fn substitute_by_poly_checked(&mut self, v: &VarId, poly: &Polynomial, guard: &SizeGuard) -> Result<(), SizeLimitExceeded> {
        let mut new_poly = Polynomial::new();
        self.terms.retain(|term, coeff| {
            if let Some(new_term) = term.remove_var(v) {
                new_poly += Polynomial::from_term(new_term, coeff.clone()) * poly;
                false
            } else {
                true
            }
        });
        self.add_assign_checked(new_poly, guard)
    }
}
