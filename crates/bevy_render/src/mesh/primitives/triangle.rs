use super::{Facing, Mesh, MeshFacingExtension, Meshable};
use crate::{mesh::Indices, render_asset::RenderAssetPersistencePolicy};
use bevy_math::{
    primitives::{Triangle2d, WindingOrder},
    Vec2,
};
use wgpu::PrimitiveTopology;

/// A builder used for creating a [`Mesh`] with a [`Triangle2d`] shape.
#[derive(Clone, Copy, Debug, Default)]
pub struct Triangle2dMeshBuilder {
    /// The [`Triangle2d`] shape.
    pub triangle: Triangle2d,
    /// The XYZ direction that the mesh is facing.
    /// The default is [`Facing::Z`].
    pub facing: Facing,
}

impl MeshFacingExtension for Triangle2dMeshBuilder {
    #[inline]
    fn facing(mut self, facing: Facing) -> Self {
        self.facing = facing;
        self
    }
}

impl Triangle2dMeshBuilder {
    /// Creates a new [`Triangle2dMesh`] from points `a`, `b`, and `c`.
    #[inline]
    pub const fn new(a: Vec2, b: Vec2, c: Vec2) -> Self {
        Self {
            triangle: Triangle2d::new(a, b, c),
            facing: Facing::Z,
        }
    }

    /// Builds a [`Mesh`] based on the configuration in `self`.
    pub fn build(&self) -> Mesh {
        let [a, b, c] = self.triangle.vertices;

        let positions = match self.facing {
            Facing::X | Facing::NegX => [[0.0, a.y, -a.x], [0.0, b.y, -b.x], [0.0, c.y, -c.x]],
            Facing::Y | Facing::NegY => [[a.x, 0.0, -a.y], [b.x, 0.0, -b.y], [c.x, 0.0, -c.y]],
            Facing::Z | Facing::NegZ => [[a.x, a.y, 0.0], [b.x, b.y, 0.0], [c.x, c.y, 0.0]],
        };

        let normals = vec![self.facing.to_array(); 3];

        // The extents of the bounding box of the triangle,
        // used to compute the UV coordinates of the points.
        let extents = a.min(b).min(c).abs().max(a.max(b).max(c)) * Vec2::new(1.0, -1.0);
        let uvs = vec![
            a / extents / 2.0 + 0.5,
            b / extents / 2.0 + 0.5,
            c / extents / 2.0 + 0.5,
        ];

        let flipped = self.facing.signum() < 0;
        let is_ccw = self.triangle.winding_order() == WindingOrder::CounterClockwise;
        let is_cw = self.triangle.winding_order() == WindingOrder::Clockwise;
        let indices = if (is_ccw && !flipped) || (is_cw && flipped) {
            Indices::U32(vec![0, 1, 2])
        } else {
            Indices::U32(vec![0, 2, 1])
        };

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetPersistencePolicy::Keep,
        )
        .with_indices(Some(indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::from(positions))
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

impl Meshable for Triangle2d {
    type Output = Triangle2dMeshBuilder;

    fn mesh(&self) -> Triangle2dMeshBuilder {
        Triangle2dMeshBuilder {
            triangle: *self,
            ..Default::default()
        }
    }
}

impl From<Triangle2d> for Mesh {
    fn from(triangle: Triangle2d) -> Self {
        triangle.mesh().build()
    }
}

impl From<Triangle2dMeshBuilder> for Mesh {
    fn from(triangle: Triangle2dMeshBuilder) -> Self {
        triangle.build()
    }
}
