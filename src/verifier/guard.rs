use crate::bipoly::{Polynomial};

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
    pub fn sub_assign_checked(&mut self, rhs: Polynomial, guard: &SizeGuard) -> Result<(), SizeLimitExceeded> {
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
            guard.check(new_terms.len())?
        }

        self.terms = new_terms;
        Ok(())
    }
}
