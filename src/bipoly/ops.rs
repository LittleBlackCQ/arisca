use super::poly::Polynomial;
use std::ops::{Add, AddAssign, Sub, SubAssign, Neg, Mul, MulAssign};

impl<'a> AddAssign<&'a Polynomial> for Polynomial {
    fn add_assign(&mut self, rhs: &'a Polynomial) {
        self.add_assign(rhs);
    }
}

impl AddAssign<Polynomial> for Polynomial {
    fn add_assign(&mut self, rhs: Polynomial) {
        self.add_assign(&rhs);
    }
}

impl<'a> SubAssign<&'a Polynomial> for Polynomial {
    fn sub_assign(&mut self, rhs: &'a Polynomial) {
        self.sub_assign(rhs);
    }
}

impl SubAssign<Polynomial> for Polynomial {
    fn sub_assign(&mut self, rhs: Polynomial) {
        self.sub_assign(&rhs);
    }
}

impl<'a> MulAssign<&'a Polynomial> for Polynomial {
    fn mul_assign(&mut self, rhs: &'a Polynomial) {
        self.mul_assign(rhs);
    }
}

impl MulAssign<Polynomial> for Polynomial {
    fn mul_assign(&mut self, rhs: Polynomial) {
        self.mul_assign(&rhs);
    }
}

impl Neg for Polynomial {
    type Output = Polynomial;
    fn neg(mut self) -> Self::Output {
        self.neg_assign();
        self
    }
}

impl<'a> Neg for &'a Polynomial {
    type Output = Polynomial;
    fn neg(self) -> Polynomial {
        let mut res = self.clone();
        res.neg_assign();
        res
    }
}

impl Add<Polynomial> for Polynomial {
    type Output = Polynomial;
    fn add(mut self, rhs: Polynomial) -> Self::Output {
        self.add_assign(&rhs);
        self
    }
}

impl<'a> Add<&'a Polynomial> for Polynomial {
    type Output = Polynomial;
    fn add(mut self, rhs: &'a Polynomial) -> Polynomial {
        self += rhs;
        self
    }
}

impl<'a> Add<Polynomial> for &'a Polynomial {
    type Output = Polynomial;
    fn add(self, mut rhs: Polynomial) -> Polynomial {
        rhs += self;
        rhs
    }
}

impl<'a, 'b> Add<&'b Polynomial> for &'a Polynomial {
    type Output = Polynomial;
    fn add(self, rhs: &'b Polynomial) -> Polynomial {
        let mut res = self.clone();
        res += rhs;
        res
    }
}

impl Sub<Polynomial> for Polynomial {
    type Output = Polynomial;
    fn sub(mut self, rhs: Polynomial) -> Self::Output {
        self.sub_assign(&rhs);
        self
    }
}

impl<'a> Sub<&'a Polynomial> for Polynomial {
    type Output = Polynomial;
    fn sub(mut self, rhs: &'a Polynomial) -> Polynomial {
        self -= rhs;
        self
    }
}

impl<'a> Sub<Polynomial> for &'a Polynomial {
    type Output = Polynomial;
    fn sub(self, rhs: Polynomial) -> Polynomial {
        let mut res = self.clone();
        res -= &rhs;
        res
    }
}

impl<'a, 'b> Sub<&'b Polynomial> for &'a Polynomial {
    type Output = Polynomial;
    fn sub(self, rhs: &'b Polynomial) -> Polynomial {
        let mut res = self.clone();
        res -= rhs;
        res
    }
}

impl Mul<Polynomial> for Polynomial {
    type Output = Polynomial;
    fn mul(mut self, rhs: Polynomial) -> Self::Output {
        self.mul_assign(&rhs);
        self
    }
}

impl<'a> Mul<&'a Polynomial> for Polynomial {
    type Output = Polynomial;
    fn mul(mut self, rhs: &'a Polynomial) -> Polynomial {
        self *= rhs;
        self
    }
}

impl<'a> Mul<Polynomial> for &'a Polynomial {
    type Output = Polynomial;
    fn mul(self, mut rhs: Polynomial) -> Polynomial {
        rhs *= self;
        rhs
    }
}

impl<'a, 'b> Mul<&'b Polynomial> for &'a Polynomial {
    type Output = Polynomial;
    fn mul(self, rhs: &'b Polynomial) -> Polynomial {
        let mut res = self.clone();
        res *= rhs;
        res
    }
}