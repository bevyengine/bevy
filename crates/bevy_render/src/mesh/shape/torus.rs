use crate::mesh::{Mesh, Meshable};

/// A torus (donut) shape.
#[deprecated(
    since = "0.13.0",
    note = "please use the `Torus` primitive in `bevy_math` instead"
)]
#[derive(Debug, Clone, Copy)]
pub struct Torus {
    pub radius: f32,
    pub ring_radius: f32,
    pub subdivisions_segments: usize,
    pub subdivisions_sides: usize,
}

impl Default for Torus {
    fn default() -> Self {
        Torus {
            radius: 1.0,
            ring_radius: 0.5,
            subdivisions_segments: 32,
            subdivisions_sides: 24,
        }
    }
}

impl From<Torus> for Mesh {
    fn from(torus: Torus) -> Self {
        bevy_math::primitives::Torus {
            minor_radius: torus.ring_radius,
            major_radius: torus.radius,
        }
        .mesh()
        .minor_resolution(torus.subdivisions_sides)
        .major_resolution(torus.subdivisions_segments)
        .build()
    }
}
