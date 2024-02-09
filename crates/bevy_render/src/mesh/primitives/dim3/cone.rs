use bevy_math::{primitives::Cone, Vec2};
use wgpu::PrimitiveTopology;

use crate::{
    mesh::{Indices, Mesh, Meshable},
    render_asset::RenderAssetUsages,
};

/// A builder used for creating a [`Mesh`] with a [`Cone`] shape.
#[derive(Clone, Copy, Debug)]
pub struct ConeMeshBuilder {
    /// The [`Cone`] shape.
    pub cone: Cone,
    /// The number of vertices used for the bottom of the cone.
    ///
    /// The default is `32`.
    pub resolution: u32,
    /// The number of segments along the height of the cone.
    /// Must be greater than `0` for geometry to be generated.
    ///
    /// The default is `1`.
    pub segments: u32,
}

impl Default for ConeMeshBuilder {
    fn default() -> Self {
        Self {
            cone: Cone::default(),
            resolution: 32,
            segments: 1,
        }
    }
}

impl ConeMeshBuilder {
    /// Creates a new [`ConeMeshBuilder`] from the given radius, a height,
    /// and a resolution used for the bottom.
    #[inline]
    pub fn new(radius: f32, height: f32, resolution: u32) -> Self {
        Self {
            cone: Cone { radius, height },
            resolution,
            ..Default::default()
        }
    }

    /// Sets the number of vertices used for the bottom of the cone.
    #[inline]
    pub const fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }

    /// Sets the number of segments along the height of the cone.
    /// Must be greater than `0` for geometry to be generated.
    #[inline]
    pub const fn segments(mut self, segments: u32) -> Self {
        self.segments = segments;
        self
    }

    /// Builds a [`Mesh`] based on the configuration in `self`.
    pub fn build(&self) -> Mesh {
        let resolution = self.resolution;
        let segments = self.segments;

        debug_assert!(resolution > 2);
        debug_assert!(segments > 0);

        let num_rings = segments + 1;
        let num_vertices = resolution * 2 + num_rings * (resolution + 1);
        let num_faces = resolution * (num_rings - 2);
        let num_indices = (2 * num_faces + 2 * (resolution - 1) * 2) * 3;

        let mut positions = Vec::with_capacity(num_vertices as usize);
        let mut normals = Vec::with_capacity(num_vertices as usize);
        let mut uvs = Vec::with_capacity(num_vertices as usize);
        let mut indices = Vec::with_capacity(num_indices as usize);

        let step_theta = std::f32::consts::TAU / resolution as f32;
        let step_y = self.cone.height / segments as f32;

        // rings
        let normal_y = Vec2::new(self.cone.radius, self.cone.height).normalize().x;
        let normal_horizontal_mul = (1.0 - normal_y * normal_y).sqrt();
        for ring in 0..num_rings {
            let y = ring as f32 * step_y;
            let radius_at_y = (self.cone.height - y) / self.cone.height * self.cone.radius;

            for segment in 0..=resolution {
                let theta = segment as f32 * step_theta;
                let (sin, cos) = theta.sin_cos();

                positions.push([radius_at_y * cos, y, radius_at_y * sin]);
                normals.push([
                    cos * normal_horizontal_mul,
                    normal_y,
                    sin * normal_horizontal_mul,
                ]);
                uvs.push([
                    segment as f32 / resolution as f32,
                    ring as f32 / segments as f32,
                ]);
            }
        }

        // barrel skin

        for i in 0..segments {
            let ring = i * (resolution + 1);
            let next_ring = (i + 1) * (resolution + 1);

            for j in 0..resolution {
                indices.extend_from_slice(&[
                    ring + j,
                    next_ring + j,
                    ring + j + 1,
                    next_ring + j,
                    next_ring + j + 1,
                    ring + j + 1,
                ]);
            }
        }

        // bottom cap

        let offset = positions.len() as u32;
        let (y, normal_y, winding, radius) = (0.0, -1., (0, 1), self.cone.radius);

        for i in 0..self.resolution {
            let theta = i as f32 * step_theta;
            let (sin, cos) = theta.sin_cos();

            positions.push([cos * radius, y, sin * radius]);
            normals.push([0.0, normal_y, 0.0]);
            uvs.push([0.5 * (cos + 1.0), 1.0 - 0.5 * (sin + 1.0)]);
        }

        for i in 1..(self.resolution - 1) {
            indices.extend_from_slice(&[offset, offset + i + winding.0, offset + i + winding.1]);
        }

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(Indices::U32(indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

impl Meshable for Cone {
    type Output = ConeMeshBuilder;

    fn mesh(&self) -> Self::Output {
        ConeMeshBuilder {
            cone: *self,
            ..Default::default()
        }
    }
}

impl From<Cone> for Mesh {
    fn from(cone: Cone) -> Self {
        cone.mesh().build()
    }
}

impl From<ConeMeshBuilder> for Mesh {
    fn from(cone: ConeMeshBuilder) -> Self {
        cone.build()
    }
}
