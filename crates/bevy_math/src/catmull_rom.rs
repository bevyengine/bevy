use glam::{Vec2, Vec3, Vec3A};

use std::{
    fmt::Debug,
    iter::Sum,
    ops::{Add, Mul, Sub},
};

/// A point in space of any dimension that supports addition and multiplication.
pub trait Point:
    Mul<f32, Output = Self>
    + Add<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Add<f32, Output = Self>
    + Sum
    + Default
    + Debug
    + Clone
    + PartialEq
    + Copy
{
}
impl Point for Vec3 {}
impl Point for Vec3A {}
impl Point for Vec2 {}

/// A spline that passes through all of its points.
///
/// The Catmull-Rom spline can be controlled with its `points`, and the `tension`. The `tension`
/// determines how closely the spline follows the linear path between points.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CatmullRom<P: Point> {
    points: Vec<P>,
    tension: f32,
    segments: Vec<Segment<P>>,
}

/// Represents a segment of a Catmull-Rom spline, used to hold precomputed coefficients for fast
/// interpolation. A segment is composed of a path and its four nearest points which influence it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Segment<P: Point> {
    coeff: [P; 4],
}

impl<P: Point> CatmullRom<P> {
    /// Construct a new Catmull-Rom spline
    pub fn new(points: impl Into<Vec<P>>, tension: f32) -> Result<Self, CatmullRomError> {
        let points = points.into();
        let segments = Self::compute_segment_coefficients(&points, tension)?;
        Ok(Self {
            points,
            tension,
            segments,
        })
    }

    /// Compute the position coordinate at `t` along the spline.
    ///
    /// Note that `t` varies from `0..=(n_points - 3)`.
    pub fn position(&self, t: f32) -> P {
        let (segment, t) = self.segment(t);
        let [a, b, c, d] = segment.coeff;
        a + b * t + c * t.powi(2) + d * t.powi(3)
    }

    /// Compute the instantaneous velocity vector at `t` along the spline.
    ///
    /// Note that `t` varies from `0..=(n_points - 3)`.
    pub fn velocity(&self, t: f32) -> P {
        let (segment, t) = self.segment(t);
        let [_, b, c, d] = segment.coeff;
        b + c * 2.0 * t + d * 3.0 * t.powi(2)
    }

    /// Compute the instantaneous acceleration vector at `t` along the spline.
    ///
    /// Note that `t` varies from `0..=(n_points - 3)`.
    pub fn acceleration(&self, t: f32) -> P {
        let (segment, t) = self.segment(t);
        let [_, _, c, d] = segment.coeff;
        c * 2.0 + d * 6.0 * t
    }

    /// Returns the [`Segment`] and local `t` value given a spline's global `t` value.
    fn segment(&self, t: f32) -> (&Segment<P>, f32) {
        let i = (t.floor() as usize).clamp(0, self.segments.len() - 1);
        (&self.segments[i], t - i as f32)
    }

    fn compute_segment_coefficients(
        points: &[P],
        tension: f32,
    ) -> Result<Vec<Segment<P>>, CatmullRomError> {
        if points.len() < 4 {
            return Err(CatmullRomError::NotEnoughPoints);
        } else {
            Ok(points
                .windows(4)
                .map(|p| Self::catmull_rom_coeff([p[0], p[1], p[2], p[3]], tension))
                .collect())
        }
    }

    fn catmull_rom_coeff(p: [P; 4], tau: f32) -> Segment<P> {
        Segment {
            coeff: [
                p[1],
                (p[0] * -tau + p[2] * tau),
                (p[0] * 2.0 * tau + p[1] * (tau - 3.0) + p[2] * (3.0 - 2.0 * tau) + p[3] * -tau),
                (p[0] * -tau + p[1] * (2.0 - tau) + p[2] * (tau - 2.0) + p[3] * tau),
            ],
        }
    }

    /// Split the Catmull-Rom spline into `subdivisions` evenly spaced `t` values across the length
    /// of the curve from t = `0..=1`, and sample with the supplied `sample_function`.
    #[inline]
    pub fn sample(&self, subdivisions: i32, sample_function: fn(&Self, f32) -> P) -> Vec<P> {
        (0..=subdivisions)
            .map(|i| {
                let t = (i as f32 / subdivisions as f32) * self.segments.len() as f32;
                sample_function(self, t)
            })
            .collect()
    }

    /// Split the Catmull-Rom spline into `subdivisions` evenly spaced `t` values across the length
    /// of the curve. sampling the position at each step.
    pub fn to_positions(&self, subdivisions: i32) -> Vec<P> {
        self.sample(subdivisions, Self::position)
    }

    /// Split the Catmull-Rom spline into `subdivisions` evenly spaced `t` values across the length
    /// of the curve. sampling the velocity at each step.
    pub fn to_velocities(&self, subdivisions: i32) -> Vec<P> {
        self.sample(subdivisions, Self::velocity)
    }

    /// Split the Catmull-Rom spline into `subdivisions` evenly spaced `t` values across the length
    /// of the curve. sampling the acceleration at each step.
    pub fn to_accelerations(&self, subdivisions: i32) -> Vec<P> {
        self.sample(subdivisions, Self::acceleration)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CatmullRomError {
    NotEnoughPoints,
}
