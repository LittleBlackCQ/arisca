use super::{poly::Polynomial, mono::Monomial};
use std::fmt;

impl fmt::Debug for Monomial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let vars = self.term();
        let coeff = self.coeff();
        if coeff < 0 {
            write!(f, "- ")?;
        }
        if coeff.abs() != 1 {
            write!(f, "{}", coeff.abs())?;
        } 
        let s: Vec<String> = vars.iter().map(|v| format!("x{}", v)).collect();
        write!(f, "{}", s.join("*"))
    }
}

impl fmt::Debug for Polynomial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let terms = self.terms();
        if terms.is_empty() {
            return write!(f, "0");
        }

        let mut first = true;
        for m in terms {
            let coeff = m.coeff();
            if first {
                write!(f, "{:?}", m)?;
                first = false;
            } else if coeff > 0 {
                write!(f, " + {:?}", m)?;
            } else {
                write!(f, " {:?}", m)?;
            }
        }
        Ok(())
    }
}
