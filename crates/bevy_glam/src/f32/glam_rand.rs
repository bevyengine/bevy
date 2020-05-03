use super::{Mat2, Mat3, Mat4, Quat, Vec2, Vec3, Vec4};

use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

impl Distribution<Mat2> for Standard {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Mat2 {
        Mat2::from_cols_array(&rng.gen())
    }
}

impl Distribution<Mat3> for Standard {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Mat3 {
        Mat3::from_cols_array(&rng.gen())
    }
}

impl Distribution<Mat4> for Standard {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Mat4 {
        Mat4::from_cols_array(&rng.gen())
    }
}

impl Distribution<Quat> for Standard {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Quat {
        use core::f32::consts::PI;
        let yaw = -PI + rng.gen::<f32>() * 2.0 * PI;
        let pitch = -PI + rng.gen::<f32>() * 2.0 * PI;
        let roll = -PI + rng.gen::<f32>() * 2.0 * PI;
        Quat::from_rotation_ypr(yaw, pitch, roll)
    }
}

impl Distribution<Vec2> for Standard {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec2 {
        rng.gen::<(f32, f32)>().into()
    }
}

impl Distribution<Vec3> for Standard {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec3 {
        rng.gen::<(f32, f32, f32)>().into()
    }
}

impl Distribution<Vec4> for Standard {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec4 {
        rng.gen::<[f32; 4]>().into()
    }
}
