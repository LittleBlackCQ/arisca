use crate::bipoly::poly::Polynomial;
use std::ops::{Add, AddAssign, Sub, SubAssign, Neg, Mul, MulAssign};

impl Add for Polynomial {
    type Output = Polynomial;
    fn add(self, rhs: Polynomial) -> Self::Output {
        let mut res = self;
        res.add_assign(&rhs);
        res
    }
}

impl AddAssign for Polynomial {
    fn add_assign(&mut self, rhs: Polynomial) {
        self.add_assign(&rhs);
    }
}

impl Sub for Polynomial {
    type Output = Polynomial;
    fn sub(self, rhs: Self) -> Self::Output {
        let mut res = self;
        res.sub_assign(&rhs);
        res
    }
}

impl SubAssign for Polynomial {
    fn sub_assign(&mut self, rhs: Polynomial) {
        self.sub_assign(&rhs);
    }
}

impl Neg for Polynomial {
    type Output = Polynomial;
    fn neg(mut self) -> Self::Output {
        self.neg_assign();
        self
    }
}

impl Mul for Polynomial {
    type Output = Polynomial;
    fn mul(self, rhs: Polynomial) -> Self::Output {
        let mut res = self;
        res.mul_assign(&rhs);
        res
    }
}

impl MulAssign for Polynomial {
    fn mul_assign(&mut self, rhs: Polynomial) {
        self.mul_assign(&rhs);
    }
}