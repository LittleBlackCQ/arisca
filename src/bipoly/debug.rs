use super::*;

use std::fmt;

impl fmt::Debug for Monomial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let vars = self.term();
        let coeff = self.coeff(); // returns &BigInt

        // Handle sign
        if coeff.is_negative() {
            write!(f, "- ")?;
        }

        // Handle coefficient magnitude
        let abs_coeff = coeff.clone().abs();
        if !(abs_coeff == 1) || vars.is_empty() {
            write!(f, "{}", abs_coeff)?;
        } 
        
        // Handle variables
        let s: Vec<String> = vars.iter().map(|v| {
            if *v > 0 { format!("x{}", v) }
            else { format!("|x{:?}|", v)}
        }).collect();
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
            let coeff = m.coeff(); // returns &BigInt
            
            if first {
                write!(f, "{:?}", m)?;
                first = false;
            } else if coeff.is_positive() {
                write!(f, " + {:?}", m)?;
            } else {
                write!(f, " {:?}", m)?;
            }
        }
        Ok(())
    }
}
