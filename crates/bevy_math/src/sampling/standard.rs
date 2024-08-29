//! This module holds local implementations of the [`Distribution`] trait for [`Standard`], which
//! allow certain Bevy math types (those whose values can be randomly generated without additional
//! input other than an [`Rng`]) to be produced using [`rand`]'s APIs. It also holds [`FromRng`],
//! an ergonomic extension to that functionality which permits the omission of type annotations.
//!
//! For instance:
//! ```
//! # use rand::{random, Rng, SeedableRng, rngs::StdRng, distributions::Standard};
//! # use bevy_math::{Dir3, sampling::FromRng};
//! let mut rng = StdRng::seed_from_u64(7313429298);
//! // Random direction using thread-local rng
//! let random_direction1: Dir3 = random();
//!
//! // Random direction using the rng constructed above
//! let random_direction2: Dir3 = rng.gen();
//!
//! // The same as the previous but with different syntax
//! let random_direction3 = Dir3::from_rng(&mut rng);
//!
//! // Five random directions, using Standard explicitly
//! let many_random_directions: Vec<Dir3> = rng.sample_iter(Standard).take(5).collect();
//! ```

use std::f32::consts::TAU;

use crate::{
    primitives::{Circle, Sphere},
    Dir2, Dir3, Dir3A, Quat, Rot2, ShapeSample, Vec3A,
};
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

/// Ergonomics trait for a type with a [`Standard`] distribution, allowing values to be generated
/// uniformly from an [`Rng`] by a method in its own namespace.
///
/// Example
/// ```
/// # use rand::{Rng, SeedableRng, rngs::StdRng};
/// # use bevy_math::{Dir3, sampling::FromRng};
/// let mut rng = StdRng::seed_from_u64(451);
/// let random_dir = Dir3::from_rng(&mut rng);
/// ```
pub trait FromRng
where
    Self: Sized,
    Standard: Distribution<Self>,
{
    /// Construct a value of this type uniformly at random using `rng` as the source of randomness.
    fn from_rng<R: Rng + ?Sized>(rng: &mut R) -> Self {
        rng.gen()
    }
}

impl Distribution<Dir2> for Standard {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Dir2 {
        let circle = Circle::new(1.0);
        let vector = circle.sample_boundary(rng);
        Dir2::new_unchecked(vector)
    }
}

impl FromRng for Dir2 {}

impl Distribution<Dir3> for Standard {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Dir3 {
        let sphere = Sphere::new(1.0);
        let vector = sphere.sample_boundary(rng);
        Dir3::new_unchecked(vector)
    }
}

impl FromRng for Dir3 {}

impl Distribution<Dir3A> for Standard {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Dir3A {
        let sphere = Sphere::new(1.0);
        let vector: Vec3A = sphere.sample_boundary(rng).into();
        Dir3A::new_unchecked(vector)
    }
}

impl FromRng for Dir3A {}

impl Distribution<Rot2> for Standard {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Rot2 {
        let angle = rng.gen_range(0.0..TAU);
        Rot2::radians(angle)
    }
}

impl FromRng for Rot2 {}

impl FromRng for Quat {}
