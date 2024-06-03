use bevy_math::{primitives::Cone, Vec3};
use wgpu::PrimitiveTopology;

use crate::{
    mesh::{Indices, Mesh, MeshBuilder, Meshable},
    render_asset::RenderAssetUsages,
};

/// Anchoring options for [`ConeMeshBuilder`]
#[derive(Debug, Copy, Clone, Default)]
pub enum ConeAnchor {
    #[default]
    /// Midpoint between the tip of the cone and the center of its base.
    MidPoint,
    /// The Tip of the triangle
    Tip,
    /// The center of the base circle
    Base,
}

/// A builder used for creating a [`Mesh`] with a [`Cone`] shape.
#[derive(Clone, Copy, Debug)]
pub struct ConeMeshBuilder {
    /// The [`Cone`] shape.
    pub cone: Cone,
    /// The number of vertices used for the base of the cone.
    ///
    /// The default is `32`.
    pub resolution: u32,
    /// The anchor point for the cone mesh, defaults to the midpoint between
    /// the tip of the cone and the center of its base
    pub anchor: ConeAnchor,
}

impl Default for ConeMeshBuilder {
    fn default() -> Self {
        Self {
            cone: Cone::default(),
            resolution: 32,
            anchor: ConeAnchor::default(),
        }
    }
}

impl ConeMeshBuilder {
    /// Creates a new [`ConeMeshBuilder`] from a given radius, height,
    /// and number of vertices used for the base of the cone.
    #[inline]
    pub const fn new(radius: f32, height: f32, resolution: u32) -> Self {
        Self {
            cone: Cone { radius, height },
            resolution,
            anchor: ConeAnchor::MidPoint,
        }
    }

    /// Sets the number of vertices used for the base of the cone.
    #[inline]
    pub const fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }

    /// Sets a custom anchor point for the mesh
    #[inline]
    pub const fn anchor(mut self, anchor: ConeAnchor) -> Self {
        self.anchor = anchor;
        self
    }
}

impl MeshBuilder for ConeMeshBuilder {
    fn build(&self) -> Mesh {
        let half_height = self.cone.height / 2.0;

        // `resolution` vertices for the base, `resolution` vertices for the bottom of the lateral surface,
        // and one vertex for the tip.
        let num_vertices = self.resolution as usize * 2 + 1;
        let num_indices = self.resolution as usize * 6 - 6;

        let mut positions = Vec::with_capacity(num_vertices);
        let mut normals = Vec::with_capacity(num_vertices);
        let mut uvs = Vec::with_capacity(num_vertices);
        let mut indices = Vec::with_capacity(num_indices);

        // Tip
        positions.push([0.0, half_height, 0.0]);

        // The tip doesn't have a singular normal that works correctly.
        // We use an invalid normal here so that it becomes NaN in the fragment shader
        // and doesn't affect the overall shading. This might seem hacky, but it's one of
        // the only ways to get perfectly smooth cones without creases or other shading artefacts.
        //
        // Note that this requires that normals are not normalized in the vertex shader,
        // as that would make the entire triangle invalid and make the cone appear as black.
        normals.push([0.0, 0.0, 0.0]);

        // The UVs of the cone are in polar coordinates, so it's like projecting a circle texture from above.
        // The center of the texture is at the center of the lateral surface, at the tip of the cone.
        uvs.push([0.5, 0.5]);

        // Now we build the lateral surface, the side of the cone.

        // The vertex normals will be perpendicular to the surface.
        //
        // Here we get the slope of a normal and use it for computing
        // the multiplicative inverse of the length of a vector in the direction
        // of the normal. This allows us to normalize vertex normals efficiently.
        let normal_slope = self.cone.radius / self.cone.height;
        // Equivalent to Vec2::new(1.0, slope).length().recip()
        let normalization_factor = (1.0 + normal_slope * normal_slope).sqrt().recip();

        // How much the angle changes at each step
        let step_theta = std::f32::consts::TAU / self.resolution as f32;

        // Add vertices for the bottom of the lateral surface.
        for segment in 0..self.resolution {
            let theta = segment as f32 * step_theta;
            let (sin, cos) = theta.sin_cos();

            // The vertex normal perpendicular to the side
            let normal = Vec3::new(cos, normal_slope, sin) * normalization_factor;

            positions.push([self.cone.radius * cos, -half_height, self.cone.radius * sin]);
            normals.push(normal.to_array());
            uvs.push([0.5 + cos * 0.5, 0.5 + sin * 0.5]);
        }

        // Add indices for the lateral surface. Each triangle is formed by the tip
        // and two vertices at the base.
        for j in 1..self.resolution {
            indices.extend_from_slice(&[0, j + 1, j]);
        }

        // Close the surface with a triangle between the tip, first base vertex, and last base vertex.
        indices.extend_from_slice(&[0, 1, self.resolution]);

        // Now we build the actual base of the cone.

        let index_offset = positions.len() as u32;

        // Add base vertices.
        for i in 0..self.resolution {
            let theta = i as f32 * step_theta;
            let (sin, cos) = theta.sin_cos();

            positions.push([cos * self.cone.radius, -half_height, sin * self.cone.radius]);
            normals.push([0.0, -1.0, 0.0]);
            uvs.push([0.5 * (cos + 1.0), 1.0 - 0.5 * (sin + 1.0)]);
        }

        // Add base indices.
        for i in 1..(self.resolution - 1) {
            indices.extend_from_slice(&[index_offset, index_offset + i, index_offset + i + 1]);
        }

        // Offset the vertex positions Y axis to match the anchor
        match self.anchor {
            ConeAnchor::Tip => positions.iter_mut().for_each(|p| p[1] -= half_height),
            ConeAnchor::Base => positions.iter_mut().for_each(|p| p[1] += half_height),
            ConeAnchor::MidPoint => (),
        };

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

#[cfg(test)]
mod tests {
    use bevy_math::{primitives::Cone, Vec2};

    use crate::mesh::{primitives::MeshBuilder, Mesh, Meshable, VertexAttributeValues};

    /// Rounds floats to handle floating point error in tests.
    fn round_floats<const N: usize>(points: &mut [[f32; N]]) {
        for point in points.iter_mut() {
            for coord in point.iter_mut() {
                let round = (*coord * 100.0).round() / 100.0;
                if (*coord - round).abs() < 0.00001 {
                    *coord = round;
                }
            }
        }
    }

    #[test]
    fn cone_mesh() {
        let mut mesh = Cone {
            radius: 0.5,
            height: 1.0,
        }
        .mesh()
        .resolution(4)
        .build();

        let Some(VertexAttributeValues::Float32x3(mut positions)) =
            mesh.remove_attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("Expected positions f32x3");
        };
        let Some(VertexAttributeValues::Float32x3(mut normals)) =
            mesh.remove_attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("Expected normals f32x3");
        };

        round_floats(&mut positions);
        round_floats(&mut normals);

        // Vertex positions
        assert_eq!(
            [
                // Tip
                [0.0, 0.5, 0.0],
                // Lateral surface
                [0.5, -0.5, 0.0],
                [0.0, -0.5, 0.5],
                [-0.5, -0.5, 0.0],
                [0.0, -0.5, -0.5],
                // Base
                [0.5, -0.5, 0.0],
                [0.0, -0.5, 0.5],
                [-0.5, -0.5, 0.0],
                [0.0, -0.5, -0.5],
            ],
            &positions[..]
        );

        // Vertex normals
        let [x, y] = Vec2::new(0.5, -1.0).perp().normalize().to_array();
        assert_eq!(
            &[
                // Tip
                [0.0, 0.0, 0.0],
                // Lateral surface
                [x, y, 0.0],
                [0.0, y, x],
                [-x, y, 0.0],
                [0.0, y, -x],
                // Base
                [0.0, -1.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, -1.0, 0.0],
            ],
            &normals[..]
        );
    }
}
