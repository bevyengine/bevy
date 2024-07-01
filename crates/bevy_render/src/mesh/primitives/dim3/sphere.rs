use std::f32::consts::PI;

use crate::{
    mesh::{Indices, Mesh, MeshBuilder, Meshable},
    render_asset::RenderAssetUsages,
};
use bevy_math::primitives::Sphere;
use hexasphere::shapes::IcoSphere;
use thiserror::Error;
use wgpu::PrimitiveTopology;

/// An error when creating an icosphere [`Mesh`] from a [`SphereMeshBuilder`].
#[derive(Clone, Copy, Debug, Error)]
pub enum IcosphereError {
    /// The icosphere has too many vertices.
    #[error("Cannot create an icosphere of {subdivisions} subdivisions due to there being too many vertices being generated: {number_of_resulting_points}. (Limited to 65535 vertices or 79 subdivisions)")]
    TooManyVertices {
        /// The number of subdivisions used. 79 is the largest allowed value for a mesh to be generated.
        subdivisions: u32,
        /// The number of vertices generated. 65535 is the largest allowed value for a mesh to be generated.
        number_of_resulting_points: u32,
    },
}

/// A type of sphere mesh.
#[derive(Clone, Copy, Debug)]
pub enum SphereKind {
    /// An icosphere, a spherical mesh that consists of similar sized triangles.
    Ico {
        /// The number of subdivisions applied.
        /// The number of faces quadruples with each subdivision.
        subdivisions: u32,
    },
    /// A UV sphere, a spherical mesh that consists of quadrilaterals
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

impl Default for SphereKind {
    fn default() -> Self {
        Self::Ico { subdivisions: 5 }
    }
}

/// A builder used for creating a [`Mesh`] with an [`Sphere`] shape.
#[derive(Clone, Copy, Debug, Default)]
pub struct SphereMeshBuilder {
    /// The [`Sphere`] shape.
    pub sphere: Sphere,
    /// The type of sphere mesh that will be built.
    pub kind: SphereKind,
}

impl SphereMeshBuilder {
    /// Creates a new [`SphereMeshBuilder`] from a radius and [`SphereKind`].
    #[inline]
    pub const fn new(radius: f32, kind: SphereKind) -> Self {
        Self {
            sphere: Sphere { radius },
            kind,
        }
    }

    /// Sets the [`SphereKind`] that will be used for building the mesh.
    #[inline]
    pub const fn kind(mut self, kind: SphereKind) -> Self {
        self.kind = kind;
        self
    }

    /// Creates an icosphere mesh with the given number of subdivisions.
    ///
    /// The number of faces quadruples with each subdivision.
    /// If there are `80` or more subdivisions, the vertex count will be too large,
    /// and an [`IcosphereError`] is returned.
    ///
    /// A good default is `5` subdivisions.
    pub fn ico(&self, subdivisions: u32) -> Result<Mesh, IcosphereError> {
        if subdivisions >= 80 {
            /*
            Number of triangles:
            N = 20

            Number of edges:
            E = 30

            Number of vertices:
            V = 12

            Number of points within a triangle (triangular numbers):
            inner(s) = (s^2 + s) / 2

            Number of points on an edge:
            edges(s) = s

            Add up all vertices on the surface:
            vertices(s) = edges(s) * E + inner(s - 1) * N + V

            Expand and simplify. Notice that the triangular number formula has roots at -1, and 0, so translating it one to the right fixes it.
            subdivisions(s) = 30s + 20((s^2 - 2s + 1 + s - 1) / 2) + 12
            subdivisions(s) = 30s + 10s^2 - 10s + 12
            subdivisions(s) = 10(s^2 + 2s) + 12

            Factor an (s + 1) term to simplify in terms of calculation
            subdivisions(s) = 10(s + 1)^2 + 12 - 10
            resulting_vertices(s) = 10(s + 1)^2 + 2
            */
            let temp = subdivisions + 1;
            let number_of_resulting_points = temp * temp * 10 + 2;
            return Err(IcosphereError::TooManyVertices {
                subdivisions,
                number_of_resulting_points,
            });
        }
        let generated = IcoSphere::new(subdivisions as usize, |point| {
            let inclination = point.y.acos();
            let azimuth = point.z.atan2(point.x);

            let norm_inclination = inclination / std::f32::consts::PI;
            let norm_azimuth = 0.5 - (azimuth / std::f32::consts::TAU);

            [norm_azimuth, norm_inclination]
        });

        let raw_points = generated.raw_points();

        let points = raw_points
            .iter()
            .map(|&p| (p * self.sphere.radius).into())
            .collect::<Vec<[f32; 3]>>();

        let normals = raw_points
            .iter()
            .copied()
            .map(Into::into)
            .collect::<Vec<[f32; 3]>>();

        let uvs = generated.raw_data().to_owned();

        let mut indices = Vec::with_capacity(generated.indices_per_main_triangle() * 20);

        for i in 0..20 {
            generated.get_indices(i, &mut indices);
        }

        let indices = Indices::U32(indices);

        Ok(Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(indices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, points)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs))
    }

    /// Creates a UV sphere [`Mesh`] with the given number of
    /// longitudinal sectors and latitudinal stacks, aka horizontal and vertical resolution.
    ///
    /// A good default is `32` sectors and `18` stacks.
    pub fn uv(&self, sectors: u32, stacks: u32) -> Mesh {
        // Largely inspired from http://www.songho.ca/opengl/gl_sphere.html

        let sectors_f32 = sectors as f32;
        let stacks_f32 = stacks as f32;
        let length_inv = 1. / self.sphere.radius;
        let sector_step = 2. * PI / sectors_f32;
        let stack_step = PI / stacks_f32;

        let n_vertices = (stacks * sectors) as usize;
        let mut vertices: Vec<[f32; 3]> = Vec::with_capacity(n_vertices);
        let mut normals: Vec<[f32; 3]> = Vec::with_capacity(n_vertices);
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(n_vertices);
        let mut indices: Vec<u32> = Vec::with_capacity(n_vertices * 2 * 3);

        for i in 0..stacks + 1 {
            let stack_angle = PI / 2. - (i as f32) * stack_step;
            let xy = self.sphere.radius * stack_angle.cos();
            let z = self.sphere.radius * stack_angle.sin();

            for j in 0..sectors + 1 {
                let sector_angle = (j as f32) * sector_step;
                let x = xy * sector_angle.cos();
                let y = xy * sector_angle.sin();

                vertices.push([x, y, z]);
                normals.push([x * length_inv, y * length_inv, z * length_inv]);
                uvs.push([(j as f32) / sectors_f32, (i as f32) / stacks_f32]);
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

impl MeshBuilder for SphereMeshBuilder {
    /// Builds a [`Mesh`] according to the configuration in `self`.
    ///
    /// # Panics
    ///
    /// Panics if the sphere is a [`SphereKind::Ico`] with a subdivision count
    /// that is greater than or equal to `80` because there will be too many vertices.
    fn build(&self) -> Mesh {
        match self.kind {
            SphereKind::Ico { subdivisions } => self.ico(subdivisions).unwrap(),
            SphereKind::Uv { sectors, stacks } => self.uv(sectors, stacks),
        }
    }
}

impl Meshable for Sphere {
    type Output = SphereMeshBuilder;

    fn mesh(&self) -> Self::Output {
        SphereMeshBuilder {
            sphere: *self,
            ..Default::default()
        }
    }
}

impl From<Sphere> for Mesh {
    fn from(sphere: Sphere) -> Self {
        sphere.mesh().build()
    }
}
