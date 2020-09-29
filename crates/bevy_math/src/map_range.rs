use crate::clamp;
use glam::{Vec2, Vec3, Vec4};
use std::ops::{Add, Div, Mul, Sub};

/// A value mapped from one range to another
///
///  Input 0.5 from the range 0-1 to 0-50 would result in 25
///  Min can be less, greater or equal than max
///  Component-wise mapping for Vec2. Vec3 and Vec4
pub fn map_range<T: MapRange>(
    input: T,
    input_min: T,
    input_max: T,
    output_min: T,
    output_max: T,
) -> T {
    let alpha = alpha_along_line(input, input_min, input_max);
    lerp(output_min, output_max, alpha)
}

/// A value mapped from one range to another, where value is clamped to input range
///
///  Input 1.5 from the range 0-1 to 0-50 would result in 50
///  Min can be less, greater or equal than max
///  Component-wise mapping for Vec2. Vec3 and Vec4
pub fn map_range_clamped<T: MapRange>(
    input: T,
    input_min: T,
    input_max: T,
    output_min: T,
    output_max: T,
) -> T {
    let alpha = alpha_along_line(input, input_min, input_max);
    let alpha = alpha.clamp(&T::zero(), &T::one());
    lerp(output_min, output_max, alpha)
}

pub trait MapRange
where
    Self: Copy
        + PartialOrd
        + PartialEq
        + Add<Output = Self>
        + Sub<Output = Self>
        + Mul<Output = Self>
        + Div<Output = Self>,
{
    fn zero() -> Self;
    fn one() -> Self;
    fn clamp(&self, min: &Self, max: &Self) -> Self;
    fn near_zero(&self) -> bool;
}

const EPSILON: f32 = 1.0e-8;
const EPSILON_SQUARED: f32 = EPSILON * EPSILON;

impl MapRange for f32 {
    fn zero() -> f32 {
        0f32
    }

    fn one() -> f32 {
        1f32
    }

    fn clamp(&self, min: &f32, max: &f32) -> f32 {
        clamp::clamp(*self, *min, *max)
    }

    fn near_zero(&self) -> bool {
        self.abs() < EPSILON
    }
}

impl MapRange for Vec2 {
    fn zero() -> Vec2 {
        Vec2::zero()
    }

    fn one() -> Vec2 {
        Vec2::one()
    }

    fn clamp(&self, min: &Vec2, max: &Vec2) -> Vec2 {
        Vec2::new(
            clamp::clamp(self.x(), min.x(), max.x()),
            clamp::clamp(self.y(), min.y(), max.y()),
        )
    }

    fn near_zero(&self) -> bool {
        self.length_squared() < EPSILON_SQUARED
    }
}

impl MapRange for Vec3 {
    fn zero() -> Vec3 {
        Vec3::zero()
    }

    fn one() -> Vec3 {
        Vec3::one()
    }

    fn clamp(&self, min: &Vec3, max: &Vec3) -> Vec3 {
        Vec3::new(
            clamp::clamp(self.x(), min.x(), max.x()),
            clamp::clamp(self.y(), min.y(), max.y()),
            clamp::clamp(self.z(), min.z(), max.z()),
        )
    }

    fn near_zero(&self) -> bool {
        self.length_squared() < EPSILON_SQUARED
    }
}

impl MapRange for Vec4 {
    fn zero() -> Vec4 {
        Vec4::zero()
    }

    fn one() -> Vec4 {
        Vec4::one()
    }

    fn clamp(&self, min: &Vec4, max: &Vec4) -> Vec4 {
        Vec4::new(
            clamp::clamp(self.x(), min.x(), max.x()),
            clamp::clamp(self.y(), min.y(), max.y()),
            clamp::clamp(self.z(), min.z(), max.z()),
            clamp::clamp(self.w(), min.w(), max.w()),
        )
    }

    fn near_zero(&self) -> bool {
        self.length_squared() < EPSILON_SQUARED
    }
}

// Alpha of input along line from input_min to input_max
fn alpha_along_line<T: MapRange>(input: T, input_min: T, input_max: T) -> T {
    let divisor = input_max - input_min;
    if divisor.near_zero() {
        if input >= input_max {
            T::one()
        } else {
            T::zero()
        }
    } else {
        (input - input_min) / divisor
    }
}

// Component-wise lerp
fn lerp<T: MapRange>(input: T, output: T, alpha: T) -> T {
    input + (output - input) * alpha
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_range_lerp() {
        let t = [-0.5, 0f32, 0.25f32, 0.5f32, 0.75f32, 1f32, 1.5f32];

        let a = 10f32;
        let b = 20f32;
        let r = [5f32, 10f32, 12.5f32, 15f32, 17.5f32, 20f32, 25f32];
        for i in 0..7 {
            assert_eq!(lerp(a, b, t[i]), r[i], "lerp when a < b && a > 0");
        }

        let a = 20f32;
        let b = 10f32;
        let r = [25f32, 20f32, 17.5f32, 15f32, 12.5f32, 10f32, 5f32];
        for i in 0..7 {
            assert_eq!(lerp(a, b, t[i]), r[i], "lerp when a > b && a > 0");
        }

        let a = -10f32;
        let b = -20f32;
        let r = [-5f32, -10f32, -12.5f32, -15f32, -17.5f32, -20f32, -25f32];
        for i in 0..7 {
            assert_eq!(lerp(a, b, t[i]), r[i], "lerp when a > b && a < 0");
        }

        let a = -20f32;
        let b = -10f32;
        let r = [-25f32, -20f32, -17.5f32, -15f32, -12.5f32, -10f32, -5f32];
        for i in 0..7 {
            assert_eq!(lerp(a, b, t[i]), r[i], "lerp when a < b && a < 0");
        }

        let a = Vec3::new(-25f32, 20f32, 17.5f32);
        let b = Vec3::new(2f32, 20f32, -17.5f32);
        assert_eq!(
            lerp(a, b, Vec3::new(0.5f32, 0.5f32, 0.5f32)),
            a.lerp(b, 0.5f32),
            "lerp eq glam lerp"
        );
    }

    #[test]
    fn test_map_range() {
        let input_min = -100f32;
        let input_max = 100f32;
        let output_min = 0f32;
        let output_max = 10f32;

        let input = 0f32;
        let output = map_range(input, input_min, input_max, output_min, output_max);
        assert_eq!(output, 5f32, "map_range eq");

        let input_min = Vec3::new(-100f32, -100f32, 100f32);
        let input_max = Vec3::new(100f32, 100f32, -100f32);
        let output_min = Vec3::new(0f32, 0f32, 0f32);
        let output_max = Vec3::new(10f32, 10f32, 10f32);

        let input = Vec3::new(0f32, -50f32, 50f32);
        let output = map_range(input, input_min, input_max, output_min, output_max);
        assert!(
            output.abs_diff_eq(Vec3::new(5f32, 2.5f32, 2.5f32), EPSILON),
            "map_range glam abs_diff_eq"
        );
    }

    #[test]
    fn test_map_range_clamped() {
        let input_min = -100f32;
        let input_max = 100f32;
        let output_min = 0f32;
        let output_max = 10f32;

        let input = 200f32;
        let output = map_range_clamped(input, input_min, input_max, output_min, output_max);
        assert_eq!(output, 10f32, "map_range_clamped eq");

        let input_min = Vec3::new(-100f32, -100f32, 100f32);
        let input_max = Vec3::new(100f32, 100f32, -100f32);
        let output_min = Vec3::new(0f32, 0f32, 0f32);
        let output_max = Vec3::new(10f32, 10f32, 10f32);

        let input = Vec3::new(200f32, -50f32, 50f32);
        let output = map_range_clamped(input, input_min, input_max, output_min, output_max);
        assert!(
            output.abs_diff_eq(Vec3::new(10f32, 2.5f32, 2.5f32), EPSILON),
            "map_range_clamped glam abs_diff_eq"
        );
    }
}
