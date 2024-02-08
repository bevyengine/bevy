use bevy_math::primitives::Sphere;

use crate::mesh::{Mesh, Meshable};

/// A sphere made of sectors and stacks.
#[deprecated(
    since = "0.13.0",
    note = "please use the `Sphere` primitive in `bevy_math` instead"
)]
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy)]
pub struct UVSphere {
    /// The radius of the sphere.
    pub radius: f32,
    /// Longitudinal sectors
    pub sectors: usize,
    /// Latitudinal stacks
    pub stacks: usize,
}

impl Default for UVSphere {
    fn default() -> Self {
        Self {
            radius: 1.0,
            sectors: 36,
            stacks: 18,
        }
    }
}

impl From<UVSphere> for Mesh {
    fn from(sphere: UVSphere) -> Self {
        Sphere::new(sphere.radius)
            .mesh()
            .uv(sphere.sectors, sphere.stacks)
    }
}
