use bevy_math::{primitives::Cone, Vec3};
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
}

impl Default for ConeMeshBuilder {
    fn default() -> Self {
        Self {
            cone: Cone::default(),
            resolution: 32,
        }
    }
}

impl ConeMeshBuilder {
    /// Creates a new [`ConeMeshBuilder`] from the given radius, a height,
    /// and a resolution used for the top and bottom.
    #[inline]
    pub fn new(radius: f32, height: f32, resolution: u32) -> Self {
        Self {
            cone: Cone::new(radius, height),
            resolution,
        }
    }

    /// Sets the number of vertices used for the top and bottom of the cone.
    #[inline]
    pub const fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }

    /// Builds a [`Mesh`] based on the configuration in `self`.
    pub fn build(&self) -> Mesh {
        let resolution = self.resolution;

        debug_assert!(resolution > 2);

        let num_vertices = resolution * 2 + resolution + 1;
        let num_indices = resolution * 3 * 2;

        let mut positions = Vec::with_capacity(num_vertices as usize);
        let mut normals = Vec::with_capacity(num_vertices as usize);
        let mut uvs = Vec::with_capacity(num_vertices as usize);
        let mut indices = Vec::with_capacity(num_indices as usize);

        let step_theta = std::f32::consts::TAU / resolution as f32;
        let half_height = self.cone.height / 2.0;

        // Center of the base
        positions.push([0.0, -half_height, 0.0]);
        normals.push([0.0, -1.0, 0.0]);
        uvs.push([0.5, 0.5]);

        // Base circle vertices
        for i in 1..=resolution {
            let theta = i as f32 * step_theta;
            let (sin, cos) = theta.sin_cos();

            positions.push([self.cone.radius * cos, -half_height, self.cone.radius * sin]);
            normals.push([0.0, -1.0, 0.0]);
            uvs.push([0.5 * (cos + 1.0), 0.5 * (sin + 1.0)]);
            indices.extend_from_slice(&[0, i, i % resolution + 1]);
        }

        let tip_idx = resolution + 1;

        // Tip of the cone
        positions.push([0.0, half_height, 0.0]);
        // The normal is zero, instead of 0.0, 1.0, 0.0 which causes bad lighting
        normals.push([0.0, 0.0, 0.0]);
        uvs.push([0.5, 0.5]);

        // Cone vertices
        for i in 1..=resolution {
            // vertex
            let theta = i as f32 * step_theta;
            let (sin, cos) = theta.sin_cos();

            positions.push([self.cone.radius * cos, -half_height, self.cone.radius * sin]);
            normals.push([cos, 0.0, sin]);
            uvs.push([0.5 * (cos + 1.0), 0.5 * (sin + 1.0)]);
        }

        // Indices for the base using fan triangulation
        for i in 1..=resolution {
            indices.extend_from_slice(&[tip_idx + i, tip_idx, tip_idx + i % resolution + 1]);
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
