use crate::mesh::{Indices, Mesh};

use super::{Facing, MeshFacingExtension, Meshable};
use bevy_math::primitives::Ellipse;
use wgpu::PrimitiveTopology;

#[derive(Clone, Copy, Debug)]
pub struct EllipseMesh {
    /// The ellipse shape.
    pub ellipse: Ellipse,
    /// The number of vertices used for the ellipse mesh.
    pub vertices: usize,
    pub facing: Facing,
}

impl Default for EllipseMesh {
    fn default() -> Self {
        Self {
            ellipse: Ellipse::default(),
            vertices: 32,
            facing: Facing::Z,
        }
    }
}

impl MeshFacingExtension for EllipseMesh {
    fn facing(mut self, facing: Facing) -> Self {
        self.facing = facing;
        self
    }
}

impl EllipseMesh {
    /// Creates a new [`EllipseMesh`] from a given half width and half height and a vertex count.
    pub const fn new(half_width: f32, half_height: f32, vertices: usize) -> Self {
        Self {
            ellipse: Ellipse {
                half_width,
                half_height,
            },
            vertices,
            facing: Facing::Z,
        }
    }

    /// Sets the number of vertices used for the ellipse mesh.
    #[doc(alias = "segments")]
    pub const fn vertices(mut self, vertices: usize) -> Self {
        self.vertices = vertices;
        self
    }

    pub fn build(&self) -> Mesh {
        let mut indices = Vec::with_capacity((self.vertices - 2) * 3);
        let mut positions = Vec::with_capacity(self.vertices);
        let mut normals = Vec::with_capacity(self.vertices);
        let mut uvs = Vec::with_capacity(self.vertices);

        let facing_coords = self.facing.to_array();
        let normal_sign = self.facing.signum() as f32;
        let step = normal_sign * std::f32::consts::TAU / self.vertices as f32;

        for i in 0..self.vertices {
            let theta = std::f32::consts::FRAC_PI_2 + i as f32 * step;
            let (sin, cos) = theta.sin_cos();
            let x = cos * self.ellipse.half_width;
            let y = sin * self.ellipse.half_height;

            let position = match self.facing {
                Facing::X | Facing::NegX => [0.0, y, -x],
                Facing::Y | Facing::NegY => [x, 0.0, -y],
                Facing::Z | Facing::NegZ => [x, y, 0.0],
            };

            positions.push(position);
            normals.push(facing_coords);
            uvs.push([0.5 * (cos + 1.0), 1.0 - 0.5 * (sin + 1.0)]);
        }

        for i in 1..(self.vertices as u32 - 1) {
            indices.extend_from_slice(&[0, i, i + 1]);
        }

        Mesh::new(PrimitiveTopology::TriangleList)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
            .with_indices(Some(Indices::U32(indices)))
    }
}

impl Meshable for Ellipse {
    type Output = EllipseMesh;

    fn mesh(&self) -> Self::Output {
        EllipseMesh {
            ellipse: *self,
            ..Default::default()
        }
    }
}

impl From<Ellipse> for Mesh {
    fn from(ellipse: Ellipse) -> Self {
        ellipse.mesh().build()
    }
}

impl From<EllipseMesh> for Mesh {
    fn from(ellipse: EllipseMesh) -> Self {
        ellipse.build()
    }
}
