use crate::mesh::{IcosphereError, Mesh, Meshable};
use bevy_math::primitives::Sphere;

/// A sphere made from a subdivided Icosahedron.
#[deprecated(
    since = "0.13.0",
    note = "please use the `Sphere` primitive in `bevy_math` instead"
)]
#[derive(Debug, Clone, Copy)]
pub struct Icosphere {
    /// The radius of the sphere.
    pub radius: f32,
    /// The number of subdivisions applied.
    pub subdivisions: usize,
}

impl Default for Icosphere {
    fn default() -> Self {
        Self {
            radius: 1.0,
            subdivisions: 5,
        }
    }
}

impl TryFrom<Icosphere> for Mesh {
    type Error = IcosphereError;

    fn try_from(sphere: Icosphere) -> Result<Self, Self::Error> {
        Sphere::new(sphere.radius).mesh().ico(sphere.subdivisions)
    }
}
