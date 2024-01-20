use crate::{
    mesh::{Indices, Mesh},
    render_asset::RenderAssetPersistencePolicy,
};

use super::{Facing, MeshFacingExtension, Meshable};
use bevy_math::{primitives::Ellipse, Vec2, Vec3};
use wgpu::PrimitiveTopology;

/// A builder used for creating a [`Mesh`] with an [`Ellipse`] shape.
#[derive(Clone, Copy, Debug)]
pub struct EllipseMeshBuilder {
    /// The [`Ellipse`] shape.
    pub ellipse: Ellipse,
    /// The number of vertices used for the ellipse mesh.
    /// The default is `32`.
    #[doc(alias = "vertices")]
    pub resolution: usize,
    /// The XYZ direction that the mesh is facing.
    /// The default is [`Facing::Z`].
    pub facing: Facing,
}

impl Default for EllipseMeshBuilder {
    fn default() -> Self {
        Self {
            ellipse: Ellipse::default(),
            resolution: 32,
            facing: Facing::Z,
        }
    }
}

impl MeshFacingExtension for EllipseMeshBuilder {
    #[inline]
    fn facing(mut self, facing: Facing) -> Self {
        self.facing = facing;
        self
    }
}

impl EllipseMeshBuilder {
    /// Creates a new [`EllipseMeshBuilder`] from a given half width and half height and a vertex count.
    #[inline]
    pub const fn new(half_width: f32, half_height: f32, resolution: usize) -> Self {
        Self {
            ellipse: Ellipse {
                half_size: Vec2::new(half_width, half_height),
            },
            resolution,
            facing: Facing::Z,
        }
    }

    /// Sets the number of vertices used for the ellipse mesh.
    #[inline]
    #[doc(alias = "vertices")]
    pub const fn resolution(mut self, resolution: usize) -> Self {
        self.resolution = resolution;
        self
    }

    /// Builds a [`Mesh`] based on the configuration in `self`.
    pub fn build(&self) -> Mesh {
        let mut indices = Vec::with_capacity((self.resolution - 2) * 3);
        let mut positions = Vec::with_capacity(self.resolution);
        let mut normals = Vec::with_capacity(self.resolution);
        let mut uvs = Vec::with_capacity(self.resolution);

        self.build_mesh_data(
            Vec3::ZERO,
            &mut indices,
            &mut positions,
            &mut normals,
            &mut uvs,
        );

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetPersistencePolicy::Keep,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_indices(Some(Indices::U32(indices)))
    }

    /// Builds the ellipse mesh and pushes the data to the given vertex attribute data sets.
    pub(super) fn build_mesh_data(
        &self,
        translation: Vec3,
        indices: &mut Vec<u32>,
        positions: &mut Vec<[f32; 3]>,
        normals: &mut Vec<[f32; 3]>,
        uvs: &mut Vec<[f32; 2]>,
    ) {
        let sides = self.resolution;
        let facing_coords = self.facing.to_array();
        let normal_sign = self.facing.signum() as f32;

        // The mesh could have existing vertices, so we add an offset to find
        // the index where the ellipse's own vertices begin.
        let index_offset = positions.len() as u32;

        // Add pi/2 so that there is a vertex at the top (sin is 1.0 and cos is 0.0)
        let start_angle = std::f32::consts::FRAC_PI_2;
        let step = normal_sign * std::f32::consts::TAU / sides as f32;

        for i in 0..sides {
            // Compute vertex position at angle theta
            let theta = start_angle + i as f32 * step;
            let (sin, cos) = theta.sin_cos();
            let x = cos * self.ellipse.half_size.x;
            let y = sin * self.ellipse.half_size.y;

            // Transform vertex position based on facing direction
            let position = match self.facing {
                Facing::X | Facing::NegX => Vec3::new(0.0, y, -x),
                Facing::Y | Facing::NegY => Vec3::new(x, 0.0, -y),
                Facing::Z | Facing::NegZ => Vec3::new(x, y, 0.0),
            };

            positions.push((position + translation).to_array());
            normals.push(facing_coords);
            uvs.push([0.5 * (cos + 1.0), 1.0 - 0.5 * (sin + 1.0)]);
        }

        for i in 1..(sides as u32 - 1) {
            indices.extend_from_slice(&[index_offset, index_offset + i, index_offset + i + 1]);
        }
    }
}

impl Meshable for Ellipse {
    type Output = EllipseMeshBuilder;

    fn mesh(&self) -> Self::Output {
        EllipseMeshBuilder {
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

impl From<EllipseMeshBuilder> for Mesh {
    fn from(ellipse: EllipseMeshBuilder) -> Self {
        ellipse.build()
    }
}
