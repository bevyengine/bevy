use crate::mesh::{Indices, Mesh};
use hexasphere::shapes::IcoSphere;
use thiserror::Error;
use wgpu::PrimitiveTopology;

/// A sphere made from a subdivided Icosahedron.
#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Error)]
pub enum FromIcosphereError {
    #[error("Cannot create an icosphere of {subdivisions} subdivisions due to there being too many vertices being generated: {number_of_resulting_points}. (Limited to 65535 vertices or 79 subdivisions)")]
    TooManyVertices {
        subdivisions: usize,
        number_of_resulting_points: usize,
    },
}

impl TryFrom<Icosphere> for Mesh {
    type Error = FromIcosphereError;

    fn try_from(sphere: Icosphere) -> Result<Self, Self::Error> {
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
            return Err(FromIcosphereError::TooManyVertices {
                subdivisions: sphere.subdivisions,
                number_of_resulting_points,
            });
        }
        let generated = IcoSphere::new(sphere.subdivisions, |point| {
            let inclination = point.y.acos();
            let azimuth = point.z.atan2(point.x);

            let norm_inclination = inclination / std::f32::consts::PI;
            let norm_azimuth = 0.5 - (azimuth / std::f32::consts::TAU);

            [norm_azimuth, norm_inclination]
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

        let uvs = generated.raw_data().to_owned();

        let mut indices = Vec::with_capacity(generated.indices_per_main_triangle() * 20);

        for i in 0..20 {
            generated.get_indices(i, &mut indices);
        }

        let indices = Indices::U32(indices);

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_indices(Some(indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, points);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        Ok(mesh)
    }
}
