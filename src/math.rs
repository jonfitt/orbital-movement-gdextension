//! Three-dimensional vector math for orbital mechanics.

use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// A three-component vector using `f64` components.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    /// X component.
    pub x: f64,
    /// Y component.
    pub y: f64,
    /// Z component.
    pub z: f64,
}

impl Vec3 {
    /// Zero vector.
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    /// Unit vector along +X.
    pub const X: Self = Self {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };

    /// Unit vector along +Y.
    pub const Y: Self = Self {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };

    /// Unit vector along +Z.
    pub const Z: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };

    /// Creates a new vector.
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Dot product.
    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Cross product.
    pub fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    /// Euclidean length.
    pub fn length(self) -> f64 {
        self.length_squared().sqrt()
    }

    /// Squared Euclidean length.
    pub fn length_squared(self) -> f64 {
        self.dot(self)
    }

    /// Returns a normalized copy, or zero if length is negligible.
    pub fn normalized(self) -> Self {
        let len = self.length();
        if len <= f64::EPSILON {
            Self::ZERO
        } else {
            self / len
        }
    }

    /// Component-wise scale by scalar.
    pub fn scale(self, scalar: f64) -> Self {
        self * scalar
    }

    /// Rotates this vector about `axis` (unit vector) by `angle_rad` (right-hand rule).
    pub fn rotate_about_axis(self, axis: Self, angle_rad: f64) -> Self {
        let axis = axis.normalized();
        let cos = angle_rad.cos();
        let sin = angle_rad.sin();
        self * cos + axis.cross(self) * sin + axis * axis.dot(self) * (1.0 - cos)
    }

    /// Projects `self` onto `direction` (need not be unit).
    pub fn project_onto(self, direction: Self) -> Self {
        let dir_len_sq = direction.length_squared();
        if dir_len_sq <= f64::EPSILON {
            return Self::ZERO;
        }
        direction * (self.dot(direction) / dir_len_sq)
    }

    /// Removes the component along `direction`.
    pub fn reject_from(self, direction: Self) -> Self {
        self - self.project_onto(direction)
    }
}

impl Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl SubAssign for Vec3 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Neg for Vec3 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl Mul<f64> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl Div<f64> for Vec3 {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Vec3;

    #[test]
    fn cross_product_is_perpendicular() {
        let a = Vec3::X;
        let b = Vec3::Y;
        let c = a.cross(b);
        assert!((c - Vec3::Z).length() < 1e-12);
    }

    #[test]
    fn rotation_preserves_length() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        let rotated = v.rotate_about_axis(Vec3::Y, std::f64::consts::FRAC_PI_2);
        assert!((v.length() - rotated.length()).abs() < 1e-12);
    }
}
