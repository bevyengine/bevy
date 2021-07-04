use crate::mesh::VertexAttributeValues;
use crate::{
    mesh::{Indices, Mesh},
    pipeline::PrimitiveTopology,
};
use bevy_math::Vec3;

/// A sphere made from a subdivided Icosahedron.
#[derive(Debug, Clone, Copy)]
pub struct Icosphere {
    /// The radius of the sphere.
    pub radius: f32,
    /// The number of subdivisions applied.
    pub subdivisions: usize,
    // TODO: Generate points/indices for a UV compatible Icosphere.
    // pub has_uvs: bool,
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

        let mut default_mesh = Mesh::new(PrimitiveTopology::TriangleList);
        default_mesh.set_indices(Some(Indices::U32(consts::INDICES.into())));
        default_mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, consts::RAW_POINTS.to_vec());

        let uvs = consts::RAW_POINTS
            .iter()
            .map(|point| {
                let point = Vec3::from(*point);
                let inclination = point.y.acos();
                let azimuth = point.z.atan2(point.x);

                let norm_inclination = inclination / std::f32::consts::PI;
                let norm_azimuth = 0.5 - (azimuth / std::f32::consts::TAU);

                [norm_azimuth, norm_inclination]
            })
            .collect::<Vec<_>>();

        default_mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

        default_mesh.subdivide(
            sphere.subdivisions,
            crate::shape::shapegen::SphereInterpolatorGroup::default(),
        );

        default_mesh.set_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            default_mesh
                .attribute(Mesh::ATTRIBUTE_POSITION)
                .unwrap()
                .clone(),
        );

        let positions = default_mesh
            .attribute_mut(Mesh::ATTRIBUTE_POSITION)
            .unwrap();

        if let VertexAttributeValues::Float32x3(positions) = positions {
            positions.iter_mut().for_each(|p| {
                *p = [
                    p[0] * sphere.radius,
                    p[1] * sphere.radius,
                    p[2] * sphere.radius,
                ]
            });
        } else {
            unreachable!();
        }

        default_mesh
    }
}

#[allow(clippy::all)]
mod consts {
    pub const RAW_POINTS: [[f32; 3]; 12] = [
        // North Pole
        [0.0, 1.0, 0.0],
        // Top Ring
        [
            0.89442719099991585541,
            0.44721359549995792770,
            0.00000000000000000000,
        ],
        [
            0.27639320225002106390,
            0.44721359549995792770,
            0.85065080835203987775,
        ],
        [
            -0.72360679774997882507,
            0.44721359549995792770,
            0.52573111211913370333,
        ],
        [
            -0.72360679774997904712,
            0.44721359549995792770,
            -0.52573111211913348129,
        ],
        [
            0.27639320225002084186,
            0.44721359549995792770,
            -0.85065080835203998877,
        ],
        // Bottom Ring
        [
            0.72360679774997871405,
            -0.44721359549995792770,
            -0.52573111211913392538,
        ],
        [
            0.72360679774997904712,
            -0.44721359549995792770,
            0.52573111211913337026,
        ],
        [
            -0.27639320225002073084,
            -0.44721359549995792770,
            0.85065080835203998877,
        ],
        [
            -0.89442719099991585541,
            -0.44721359549995792770,
            0.00000000000000000000,
        ],
        [
            -0.27639320225002139697,
            -0.44721359549995792770,
            -0.85065080835203976672,
        ],
        // South Pole
        [0.0, -1.0, 0.0],
    ];

    pub const INDICES: [u32; 60] = [
        0, 2, 1, 0, 3, 2, 0, 4, 3, 0, 5, 4, 0, 1, 5, 5, 1, 6, 1, 2, 7, 2, 3, 8, 3, 4, 9, 4, 5, 10,
        5, 6, 10, 1, 7, 6, 2, 8, 7, 3, 9, 8, 4, 10, 9, 10, 6, 11, 6, 7, 11, 7, 8, 11, 8, 9, 11, 9,
        10, 11,
    ];
}
