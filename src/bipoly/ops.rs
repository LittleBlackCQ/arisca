use crate::bipoly::poly::Polynomial;
use std::ops::{Add, Mul};

impl Add for Polynomial {
    type Output = Polynomial;
    
    fn add(self, rhs: Polynomial) -> Polynomial {
        let mut res = self;
        res.add_assign(&rhs);
        res
    }
}

impl Mul for Polynomial {
    type Output = Polynomial;
    
    fn mul(self, rhs: Polynomial) -> Polynomial {
        let mut res = self;
        res.mul_assign(&rhs);
        res
    }
}