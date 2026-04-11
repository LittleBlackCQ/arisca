use super::*;

use std::fmt;

impl fmt::Debug for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let vars = self.vars();

        let s: Vec<String> = vars
            .iter()
            .map(|v| {
                if *v >= 0 {
                    format!("x{}", v)
                } else {
                    format!("|x{:?}|", v)
                }
            })
            .collect();
        write!(f, "{}", s.join("*"))
    }
}

impl fmt::Debug for Polynomial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_zero() {
            return write!(f, "0");
        }

        let mut first = true;
        for (term, coeff) in self.terms() {
            if coeff.is_positive() && !first {
                write!(f, "+{:?}{:?}", coeff, term)?;
            } else {
                write!(f, "{:?}{:?}", coeff, term)?;
            }
            first = false;
        }
        Ok(())
    }
}
