use super::{poly::Polynomial, mono::Monomial};
use std::fmt;

impl fmt::Debug for Monomial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let vars = self.vars();
        if vars.is_empty() {
            write!(f, "1")
        } else {
            let s: Vec<String> = vars.iter().map(|v| format!("x{}", v)).collect();
            write!(f, "{}", s.join("*"))
        }
    }
}

impl fmt::Debug for Polynomial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let terms = self.terms();
        if terms.is_empty() {
            return write!(f, "0");
        }
        let mut term_vec: Vec<(&Monomial, &i64)> = terms.iter().collect();
        term_vec.sort_by(|a, b| a.0.vars().cmp(&b.0.vars()));

        let mut first = true;

        for (m, coef) in term_vec {
            if first {
                if *coef < 0 { write!(f, "-")?; }
            } else {
                write!(f, " {} ", if *coef > 0 { "+" } else { "-" })?;
            }
            let abs_coef = coef.abs();
            let show_coef = abs_coef != 1 || m.vars().is_empty();
            if show_coef { write!(f, "{}", abs_coef)?; }
            if !m.vars().is_empty() {
                if show_coef { write!(f, "*")?; }
                write!(f, "{:?}", m)?;
            }
            first = false;
        }
        Ok(())
    }
}
