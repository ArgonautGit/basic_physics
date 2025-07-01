use std::ops::*;
use crate::units::*;

#[derive(Clone, Copy, Debug)]
pub struct Vector(f32, f32);

impl Vector {
    pub fn new(x: f32, y: f32) -> Self {
        Vector {
            0: x,
            1: y,
        }
    }
}

impl Default for Vector {
    fn default() -> Self {
        Vector(0.0, 0.0)
    }
}

impl AddAssign for Vector {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

impl Add for Vector {
    type Output = Vector;

    fn add(self, rhs: Self) -> Self::Output {
        let mut vector = Vector::default();
        vector.0 += rhs.0;
        vector.1 += rhs.1;
        vector
    }
}

impl SubAssign for Vector {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
    }
}

impl Sub for Vector {
    type Output = Vector;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut vector = Vector::default();
        vector.0 -= rhs.0;
        vector.1 -= rhs.1;
        vector
    }
}

impl MulAssign<Scalar> for Vector {
    fn mul_assign(&mut self, rhs: Scalar) {
        self.0 *= rhs;
        self.1 *= rhs;
    }
}

impl Mul<Scalar> for Vector {
    type Output = Vector;

    fn mul(self, rhs: Scalar) -> Self::Output {
        let mut vector = Vector::default();
        vector.0 = self.0 / rhs;
        vector.1 = self.1 / rhs;
        vector
    }
}

impl DivAssign<Scalar> for Vector {
    fn div_assign(&mut self, rhs: Scalar) {
        self.0 /= rhs;
        self.1 /= rhs;
    }
}

impl Div<Scalar> for Vector {
    type Output = Vector;

    fn div(self, rhs: Scalar) -> Self::Output {
        let mut vector = Vector::default();
        vector.0 = self.0 / rhs;
        vector.1 = self.1 / rhs;
        vector
    }
}