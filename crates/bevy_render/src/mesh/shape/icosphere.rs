use bevy_hexasphere::{shapes::IcoSphere, Vec3 as V3};

use bevy_math::Vec3;

use crate::{
    mesh::{Indices, Mesh},
    pipeline::PrimitiveTopology,
};
use std::ops::{Add, Mul};

#[derive(Copy, Clone)]
struct Vec3Hack(Vec3);

impl Add<Vec3Hack> for Vec3Hack {
    type Output = Self;

    fn add(self, rhs: Vec3Hack) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Mul<f32> for Vec3Hack {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl V3 for Vec3Hack {
    const ZERO: Self = Vec3Hack(Vec3::ZERO);

    #[inline]
    fn dot(self, other: Self) -> f32 {
        self.0.dot(other.0)
    }

    #[inline]
    fn normalize(self) -> Self {
        Self(self.0.normalize())
    }

    #[inline]
    fn from_arr3([x, y, z]: [f32; 3]) -> Self {
        Self(Vec3::new(x, y, z))
    }
}

impl From<Vec3Hack> for [f32; 3] {
    fn from(Vec3Hack(v): Vec3Hack) -> Self {
        [v.x, v.y, v.z]
    }
}

/// A sphere made from a subdivided Icosahedron.
#[derive(Debug)]
pub struct Icosphere {
    /// The radius of the sphere.
    pub radius: f32,
    /// The number of subdivisions applied.
    pub subdivisions: usize,
}

impl Default for Icosphere {
    fn default() -> Self {
        Self {
            radius: 1.0,
            subdivisions: 5,
        }
    }
}

impl From<Icosphere> for Mesh {
    fn from(sphere: Icosphere) -> Self {
        if sphere.subdivisions >= 80 {
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
            let temp = sphere.subdivisions + 1;
            let number_of_resulting_points = temp * temp * 10 + 2;

            panic!(
                "Cannot create an icosphere of {} subdivisions due to there being too many vertices being generated: {}. (Limited to 65535 vertices or 79 subdivisions)",
                sphere.subdivisions,
                number_of_resulting_points
            );
        }
        let generated = IcoSphere::new(sphere.subdivisions, |point: Vec3Hack| {
            let point = point.0;
            let inclination = point.z.acos();
            let azumith = point.y.atan2(point.x);

            let norm_inclination = 1.0 - (inclination / std::f32::consts::PI);
            let norm_azumith = (azumith / std::f32::consts::PI) * 0.5;

            [norm_inclination, norm_azumith]
        });

        let raw_points = generated.raw_points();

        let points = raw_points
            .iter()
            .map(|&p| (p * sphere.radius).into())
            .collect::<Vec<[f32; 3]>>();

        let normals = raw_points
            .iter()
            .copied()
            .map(Into::into)
            .collect::<Vec<[f32; 3]>>();

        let uvs = generated.raw_user_data().to_owned();

        let indices = generated.get_all_indices();

        let indices = Indices::U32(indices);

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_indices(Some(indices));
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, points);
        mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh
    }
}
