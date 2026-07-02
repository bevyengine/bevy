use crate::{Indices, Mesh, MeshBuilder, Meshable, PrimitiveTopology};
use bevy_asset::RenderAssetUsages;
use bevy_math::{ops, primitives::Ellipsoid};
use bevy_reflect::prelude::*;
use core::f32::consts::PI;
use glam::Vec3;

/// A type of ellipsoid mesh.
#[derive(Clone, Copy, Debug, Reflect)]
#[reflect(Default, Debug, Clone)]
pub enum EllipsoidKind {
    /// A UV ellipsoid, a spherical mesh that consists of quadrilaterals
    /// apart from triangles at the top and bottom.
    Uv {
        /// The number of longitudinal sectors, aka the horizontal resolution.
        #[doc(alias = "horizontal_resolution")]
        sectors: u32,
        /// The number of latitudinal stacks, aka the vertical resolution.
        #[doc(alias = "vertical_resolution")]
        stacks: u32,
    },
}

impl Default for EllipsoidKind {
    fn default() -> Self {
        Self::Uv {
            sectors: 10,
            stacks: 10,
        }
    }
}

/// A builder used for creating a [`Mesh`] with an [`Ellipsoid`] shape.
#[derive(Clone, Copy, Debug, Default, Reflect)]
#[reflect(Default, Debug, Clone)]
pub struct EllipsoidMeshBuilder {
    /// The [`Ellipsoid`] shape.
    pub ellipsoid: Ellipsoid,
    /// The type of ellipsoid mesh that will be built.
    pub kind: EllipsoidKind,
}

impl EllipsoidMeshBuilder {
    /// Creates a new [`EllipsoidMeshBuilder`] from a radius and [`EllipsoidKind`].
    #[inline]
    pub const fn new(radius: Vec3, kind: EllipsoidKind) -> Self {
        Self {
            ellipsoid: Ellipsoid { radii: radius },
            kind,
        }
    }

    /// Sets the [`EllipsoidKind`] that will be used for building the mesh.
    #[inline]
    pub const fn kind(mut self, kind: EllipsoidKind) -> Self {
        self.kind = kind;
        self
    }

    /// Creates a UV ellipsoid [`Mesh`] with the given number of
    /// longitudinal sectors and latitudinal stacks, aka horizontal and vertical resolution.
    ///
    /// A good default is `32` sectors and `18` stacks.
    #[expect(
        clippy::explicit_counter_loop,
        reason = "Clippy suggestion was much less clear."
    )]
    pub fn uv(&self, sectors: u32, stacks: u32) -> Mesh {
        // Largely inspired from http://www.songho.ca/opengl/gl_ellipsoid.html

        let sectors_f32 = sectors as f32;
        let stacks_f32 = stacks as f32;
        let sector_step = 2. * PI / sectors_f32;
        let stack_step = PI / stacks_f32;

        let n_vertices = (stacks * sectors) as usize;
        let mut vertices: Vec<[f32; 3]> = Vec::with_capacity(n_vertices);
        let mut normals: Vec<[f32; 3]> = Vec::with_capacity(n_vertices);
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(n_vertices);
        let mut indices: Vec<u32> = Vec::with_capacity(n_vertices * 2 * 3);

        let a = self.ellipsoid.radii.x;
        let b = self.ellipsoid.radii.y;
        let c = self.ellipsoid.radii.z;

        for i in 0..=stacks {
            let stack_angle = PI / 2.0 - (i as f32) * stack_step;

            let cos_stack = ops::cos(stack_angle);
            let sin_stack = ops::sin(stack_angle);

            for j in 0..=sectors {
                let sector_angle = j as f32 * sector_step;

                let cos_sector = ops::cos(sector_angle);
                let sin_sector = ops::sin(sector_angle);

                let x = a * cos_stack * cos_sector;
                let y = b * cos_stack * sin_sector;
                let z = c * sin_stack;

                vertices.push([x, y, z]);

                let n = Vec3::new(x / (a * a), y / (b * b), z / (c * c)).normalize();

                normals.push(n.to_array());

                uvs.push([j as f32 / sectors_f32, i as f32 / stacks_f32]);
            }
        }

        // indices
        //  k1--k1+1
        //  |  / |
        //  | /  |
        //  k2--k2+1
        for i in 0..stacks {
            let mut k1 = i * (sectors + 1);
            let mut k2 = k1 + sectors + 1;
            for _j in 0..sectors {
                if i != 0 {
                    indices.push(k1);
                    indices.push(k2);
                    indices.push(k1 + 1);
                }
                if i != stacks - 1 {
                    indices.push(k1 + 1);
                    indices.push(k2);
                    indices.push(k2 + 1);
                }
                k1 += 1;
                k2 += 1;
            }
        }

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(Indices::U32(indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

impl MeshBuilder for EllipsoidMeshBuilder {
    /// Builds a [`Mesh`] according to the configuration in `self`.
    ///
    /// # Panics
    ///
    /// Panics if the ellipsoid is a [`EllipsoidKind::Ico`] with a subdivision count
    /// that is greater than or equal to `80` because there will be too many vertices.
    fn build(&self) -> Mesh {
        match self.kind {
            EllipsoidKind::Uv { sectors, stacks } => self.uv(sectors, stacks),
        }
    }
}

impl Meshable for Ellipsoid {
    type Output = EllipsoidMeshBuilder;

    fn mesh(&self) -> Self::Output {
        EllipsoidMeshBuilder {
            ellipsoid: *self,
            ..Default::default()
        }
    }
}

impl From<Ellipsoid> for Mesh {
    fn from(ellipsoid: Ellipsoid) -> Self {
        ellipsoid.mesh().build()
    }
}
