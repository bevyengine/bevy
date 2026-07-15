//! This module holds local implementations of the [`Distribution`] trait for [`StandardUniform`], which
//! allow certain Bevy math types (those whose values can be randomly generated without additional
//! input other than an [`RngExt`]) to be produced using [`rand`]'s APIs. It also holds [`FromRng`],
//! an ergonomic extension to that functionality which permits the omission of type annotations.
//!
//! For instance:
//! ```
//! # use rand::{random,  RngExt, SeedableRng, rngs::StdRng, distr::StandardUniform};
//! # use bevy_math::{Dir3, sampling::FromRng};
//! let mut rng = StdRng::seed_from_u64(7313429298);
//! // Random direction using thread-local rng
//! let random_direction1: Dir3 = random();
//!
//! // Random direction using the rng constructed above
//! let random_direction2: Dir3 = rng.random();
//!
//! // The same as the previous but with different syntax
//! let random_direction3 = Dir3::from_rng(&mut rng);
//!
//! // Five random directions, using StandardUniform explicitly
//! let many_random_directions: Vec<Dir3> = rng.sample_iter(StandardUniform).take(5).collect();
//! ```

use core::f32::consts::{PI, TAU};

use crate::{ops, Dir2, Dir3, Dir3A, Quat, Rot2};
use glam::{Vec2, Vec3};
use rand::{
    distr::{Distribution, StandardUniform},
    RngExt,
};

/// Ergonomics trait for a type with a [`StandardUniform`] distribution, allowing values to be generated
/// uniformly from an [`RngExt`] by a method in its own namespace.
///
/// Example
/// ```
/// # use rand::{RngExt, SeedableRng, rngs::StdRng};
/// # use bevy_math::{Dir3, sampling::FromRng};
/// let mut rng = StdRng::seed_from_u64(451);
/// let random_dir = Dir3::from_rng(&mut rng);
/// ```
pub trait FromRng
where
    Self: Sized,
    StandardUniform: Distribution<Self>,
{
    /// Construct a value of this type uniformly at random using `rng` as the source of randomness.
    fn from_rng<R: RngExt + ?Sized>(rng: &mut R) -> Self {
        rng.random()
    }
}

fn sample_unit_circle<R: RngExt + ?Sized>(rng: &mut R) -> Vec2 {
    let theta = rng.random_range(0.0..TAU);
    let (sin, cos) = ops::sin_cos(theta);
    Vec2::new(cos, sin)
}

impl Distribution<Dir2> for StandardUniform {
    #[inline]
    fn sample<R: RngExt + ?Sized>(&self, rng: &mut R) -> Dir2 {
        Dir2::new_unchecked(sample_unit_circle(rng))
    }
}

impl FromRng for Dir2 {}

fn sample_unit_sphere<R: RngExt + ?Sized>(rng: &mut R) -> Vec3 {
    let z = rng.random_range(-1f32..=1f32);
    let (a_sin, a_cos) = ops::sin_cos(rng.random_range(-PI..=PI));
    let c = ops::sqrt(1f32 - z * z);
    let x = a_sin * c;
    let y = a_cos * c;
    Vec3::new(x, y, z)
}

impl Distribution<Dir3> for StandardUniform {
    #[inline]
    fn sample<R: RngExt + ?Sized>(&self, rng: &mut R) -> Dir3 {
        Dir3::new_unchecked(sample_unit_sphere(rng))
    }
}

impl FromRng for Dir3 {}

impl Distribution<Dir3A> for StandardUniform {
    #[inline]
    fn sample<R: RngExt + ?Sized>(&self, rng: &mut R) -> Dir3A {
        Dir3A::new_unchecked(sample_unit_sphere(rng).into())
    }
}

impl FromRng for Dir3A {}

impl Distribution<Rot2> for StandardUniform {
    #[inline]
    fn sample<R: RngExt + ?Sized>(&self, rng: &mut R) -> Rot2 {
        let angle = rng.random_range(0.0..TAU);
        Rot2::radians(angle)
    }
}

impl FromRng for Rot2 {}

impl FromRng for Quat {}
