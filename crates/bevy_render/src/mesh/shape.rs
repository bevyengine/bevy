use super::{Indices, Mesh};
use crate::pipeline::PrimitiveTopology;
use bevy_math::*;
use hexasphere::shapes::IcoSphere;

pub struct Cube {
    pub size: f32,
}

impl Cube {
    pub fn new(size: f32) -> Cube {
        Cube { size }
    }
}

impl Default for Cube {
    fn default() -> Self {
        Cube { size: 1.0 }
    }
}

impl From<Cube> for Mesh {
    fn from(cube: Cube) -> Self {
        Box::new(cube.size, cube.size, cube.size).into()
    }
}

pub struct Box {
    pub min_x: f32,
    pub max_x: f32,

    pub min_y: f32,
    pub max_y: f32,

    pub min_z: f32,
    pub max_z: f32,
}

impl Box {
    pub fn new(x_length: f32, y_length: f32, z_length: f32) -> Box {
        Box {
            max_x: x_length / 2.0,
            min_x: -x_length / 2.0,
            max_y: y_length / 2.0,
            min_y: -y_length / 2.0,
            max_z: z_length / 2.0,
            min_z: -z_length / 2.0,
        }
    }
}

impl Default for Box {
    fn default() -> Self {
        Box::new(2.0, 1.0, 1.0)
    }
}

impl From<Box> for Mesh {
    fn from(sp: Box) -> Self {
        let vertices = &[
            // Top
            ([sp.min_x, sp.min_y, sp.max_z], [0., 0., 1.0], [0., 0.]),
            ([sp.max_x, sp.min_y, sp.max_z], [0., 0., 1.0], [1.0, 0.]),
            ([sp.max_x, sp.max_y, sp.max_z], [0., 0., 1.0], [1.0, 1.0]),
            ([sp.min_x, sp.max_y, sp.max_z], [0., 0., 1.0], [0., 1.0]),
            // Bottom
            ([sp.min_x, sp.max_y, sp.min_z], [0., 0., -1.0], [1.0, 0.]),
            ([sp.max_x, sp.max_y, sp.min_z], [0., 0., -1.0], [0., 0.]),
            ([sp.max_x, sp.min_y, sp.min_z], [0., 0., -1.0], [0., 1.0]),
            ([sp.min_x, sp.min_y, sp.min_z], [0., 0., -1.0], [1.0, 1.0]),
            // Right
            ([sp.max_x, sp.min_y, sp.min_z], [1.0, 0., 0.], [0., 0.]),
            ([sp.max_x, sp.max_y, sp.min_z], [1.0, 0., 0.], [1.0, 0.]),
            ([sp.max_x, sp.max_y, sp.max_z], [1.0, 0., 0.], [1.0, 1.0]),
            ([sp.max_x, sp.min_y, sp.max_z], [1.0, 0., 0.], [0., 1.0]),
            // Left
            ([sp.min_x, sp.min_y, sp.max_z], [-1.0, 0., 0.], [1.0, 0.]),
            ([sp.min_x, sp.max_y, sp.max_z], [-1.0, 0., 0.], [0., 0.]),
            ([sp.min_x, sp.max_y, sp.min_z], [-1.0, 0., 0.], [0., 1.0]),
            ([sp.min_x, sp.min_y, sp.min_z], [-1.0, 0., 0.], [1.0, 1.0]),
            // Front
            ([sp.max_x, sp.max_y, sp.min_z], [0., 1.0, 0.], [1.0, 0.]),
            ([sp.min_x, sp.max_y, sp.min_z], [0., 1.0, 0.], [0., 0.]),
            ([sp.min_x, sp.max_y, sp.max_z], [0., 1.0, 0.], [0., 1.0]),
            ([sp.max_x, sp.max_y, sp.max_z], [0., 1.0, 0.], [1.0, 1.0]),
            // Back
            ([sp.max_x, sp.min_y, sp.max_z], [0., -1.0, 0.], [0., 0.]),
            ([sp.min_x, sp.min_y, sp.max_z], [0., -1.0, 0.], [1.0, 0.]),
            ([sp.min_x, sp.min_y, sp.min_z], [0., -1.0, 0.], [1.0, 1.0]),
            ([sp.max_x, sp.min_y, sp.min_z], [0., -1.0, 0.], [0., 1.0]),
        ];

        let mut positions = Vec::with_capacity(24);
        let mut normals = Vec::with_capacity(24);
        let mut uvs = Vec::with_capacity(24);

        for (position, normal, uv) in vertices.iter() {
            positions.push(*position);
            normals.push(*normal);
            uvs.push(*uv);
        }

        let indices = Indices::U32(vec![
            0, 1, 2, 2, 3, 0, // top
            4, 5, 6, 6, 7, 4, // bottom
            8, 9, 10, 10, 11, 8, // right
            12, 13, 14, 14, 15, 12, // left
            16, 17, 18, 18, 19, 16, // front
            20, 21, 22, 22, 23, 20, // back
        ]);

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(indices));
        mesh
    }
}

/// A rectangle on the XY plane.
#[derive(Debug)]
pub struct Quad {
    /// Full width and height of the rectangle.
    pub size: Vec2,
    /// Flips the texture coords of the resulting vertices.
    pub flip: bool,
}

impl Quad {
    pub fn new(size: Vec2) -> Self {
        Self { size, flip: false }
    }

    pub fn flipped(size: Vec2) -> Self {
        Self { size, flip: true }
    }
}

impl From<Quad> for Mesh {
    fn from(quad: Quad) -> Self {
        let extent_x = quad.size.x / 2.0;
        let extent_y = quad.size.y / 2.0;

        let north_west = vec2(-extent_x, extent_y);
        let north_east = vec2(extent_x, extent_y);
        let south_west = vec2(-extent_x, -extent_y);
        let south_east = vec2(extent_x, -extent_y);
        let vertices = if quad.flip {
            [
                (
                    [south_east.x, south_east.y, 0.0],
                    [0.0, 0.0, 1.0],
                    [1.0, 1.0],
                ),
                (
                    [north_east.x, north_east.y, 0.0],
                    [0.0, 0.0, 1.0],
                    [1.0, 0.0],
                ),
                (
                    [north_west.x, north_west.y, 0.0],
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0],
                ),
                (
                    [south_west.x, south_west.y, 0.0],
                    [0.0, 0.0, 1.0],
                    [0.0, 1.0],
                ),
            ]
        } else {
            [
                (
                    [south_west.x, south_west.y, 0.0],
                    [0.0, 0.0, 1.0],
                    [0.0, 1.0],
                ),
                (
                    [north_west.x, north_west.y, 0.0],
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0],
                ),
                (
                    [north_east.x, north_east.y, 0.0],
                    [0.0, 0.0, 1.0],
                    [1.0, 0.0],
                ),
                (
                    [south_east.x, south_east.y, 0.0],
                    [0.0, 0.0, 1.0],
                    [1.0, 1.0],
                ),
            ]
        };

        let indices = Indices::U32(vec![0, 2, 1, 0, 3, 2]);

        let mut positions = Vec::<[f32; 3]>::new();
        let mut normals = Vec::<[f32; 3]>::new();
        let mut uvs = Vec::<[f32; 2]>::new();
        for (position, normal, uv) in vertices.iter() {
            positions.push(*position);
            normals.push(*normal);
            uvs.push(*uv);
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_indices(Some(indices));
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh
    }
}

/// A square on the XZ plane.
#[derive(Debug)]
pub struct Plane {
    /// The total side length of the square.
    pub size: f32,
}

impl From<Plane> for Mesh {
    fn from(plane: Plane) -> Self {
        let extent = plane.size / 2.0;

        let vertices = [
            ([extent, 0.0, -extent], [0.0, 1.0, 0.0], [1.0, 1.0]),
            ([extent, 0.0, extent], [0.0, 1.0, 0.0], [1.0, 0.0]),
            ([-extent, 0.0, extent], [0.0, 1.0, 0.0], [0.0, 0.0]),
            ([-extent, 0.0, -extent], [0.0, 1.0, 0.0], [0.0, 1.0]),
        ];

        let indices = Indices::U32(vec![0, 2, 1, 0, 3, 2]);

        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        for (position, normal, uv) in vertices.iter() {
            positions.push(*position);
            normals.push(*normal);
            uvs.push(*uv);
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_indices(Some(indices));
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh
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
            // https://oeis.org/A005901
            let subdivisions = sphere.subdivisions + 1;
            let number_of_resulting_points = (subdivisions * subdivisions * 10) + 2;

            panic!(
                "Cannot create an icosphere of {} subdivisions due to there being too many vertices being generated: {}. (Limited to 65535 vertices or 79 subdivisions)",
                sphere.subdivisions,
                number_of_resulting_points
            );
        }
        let generated = IcoSphere::new(sphere.subdivisions, |point| {
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

        let uvs = generated.raw_data().to_owned();

        let mut indices = Vec::with_capacity(generated.indices_per_main_triangle() * 20);

        for i in 0..20 {
            generated.get_indices(i, &mut indices);
        }

        let indices = Indices::U32(indices);

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_indices(Some(indices));
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, points);
        mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh
    }
}

/// A torus (donut) shape
#[derive(Debug)]
pub struct Torus {
    pub radius: f32,
    pub tube_radius: f32,
    pub subdivisions_segments: usize,
    pub subdivisions_sides: usize,
}

impl Default for Torus {
    fn default() -> Self {
        Torus {
            radius: 1.0,
            tube_radius: 0.5,
            subdivisions_segments: 32,
            subdivisions_sides: 24,
        }
    }
}

impl From<Torus> for Mesh {
    fn from(torus: Torus) -> Self {
        // code adapted from http://wiki.unity3d.com/index.php/ProceduralPrimitives#C.23_-_Torus

        let n_vertices = (torus.subdivisions_segments + 1) * (torus.subdivisions_sides + 1);
        let mut positions: Vec<[f32; 3]> = Vec::with_capacity(n_vertices);
        let mut normals: Vec<[f32; 3]> = Vec::with_capacity(n_vertices);
        let mut uvs: Vec<[f32; 2]> = Vec::new();

        for segment in 0..=torus.subdivisions_segments {
            let t1 =
                segment as f32 / torus.subdivisions_segments as f32 * 2.0 * std::f32::consts::PI;
            let r1 = Vec3::new(t1.cos() * torus.radius, 0.0, t1.sin() * torus.radius);

            for side in 0..=torus.subdivisions_sides {
                let t2 = side as f32 / torus.subdivisions_sides as f32 * 2.0 * std::f32::consts::PI;
                let r2 = Quat::from_axis_angle(Vec3::unit_y(), -t1)
                    * Vec3::new(
                        t2.sin() * torus.tube_radius,
                        t2.cos() * torus.tube_radius,
                        0.0,
                    );

                let position = r1 + r2;
                let normal = r1.cross(Vec3::unit_y()).normalize();
                let uv = [
                    segment as f32 / torus.subdivisions_segments as f32,
                    side as f32 / torus.subdivisions_sides as f32,
                ];

                positions.push(position.into());
                normals.push(normal.into());
                uvs.push(uv);
            }
        }

        let n_faces = (torus.subdivisions_segments + 1) * (torus.subdivisions_sides);
        let n_triangles = n_faces * 2;
        let n_indices = n_triangles * 3;

        let mut indices: Vec<u32> = Vec::with_capacity(n_indices);

        for segment in 0..=torus.subdivisions_segments as u32 {
            for side in 0..torus.subdivisions_sides as u32 {
                let current = side + segment * (torus.subdivisions_sides as u32 + 1);

                let next = if segment < torus.subdivisions_segments as u32 {
                    (segment + 1) * (torus.subdivisions_sides as u32 + 1)
                } else {
                    0
                } + side;

                indices.push(current);
                indices.push(next);
                indices.push(next + 1);

                indices.push(current);
                indices.push(next + 1);
                indices.push(current + 1);
            }
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_indices(Some(Indices::U32(indices)));
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh
    }
}

pub struct Capsule {
    pub radius: f32,
    pub rings: usize,
    pub depth: f32,
    pub latitudes: usize,
    pub longitudes: usize,
    pub uv_profile: CapsuleUvProfile,
}
impl Default for Capsule {
    fn default() -> Self {
        Capsule {
            radius: 0.5,
            rings: 0,
            depth: 1.0,
            latitudes: 16,
            longitudes: 32,
            uv_profile: CapsuleUvProfile::Aspect,
        }
    }
}

#[derive(Clone, Copy)]
pub enum CapsuleUvProfile {
    Aspect,
    Uniform,
    Fixed,
}

impl From<Capsule> for Mesh {
    fn from(capsule: Capsule) -> Self {
        let Capsule {
            radius,
            rings,
            depth,
            latitudes,
            longitudes,
            uv_profile,
        } = capsule;

        let calc_middle = rings > 0;
        let half_lats = latitudes / 2;
        let half_latsn1 = half_lats - 1;
        let half_latsn2 = half_lats - 2;
        let ringsp1 = rings + 1;
        let lonsp1 = longitudes + 1;
        let half_depth = depth * 0.5;
        let summit = half_depth + radius;

        // Vertex index offsets.
        let vert_offset_north_hemi = longitudes;
        let vert_offset_north_equator = vert_offset_north_hemi + lonsp1 * half_latsn1;
        let vert_offset_cylinder = vert_offset_north_equator + lonsp1;
        let vert_offset_south_equator = if calc_middle {
            vert_offset_cylinder + lonsp1 * rings
        } else {
            vert_offset_cylinder
        };
        let vert_offset_south_hemi = vert_offset_south_equator + lonsp1;
        let vert_offset_south_polar = vert_offset_south_hemi + lonsp1 * half_latsn2;
        let vert_offset_south_cap = vert_offset_south_polar + lonsp1;

        // Initialize arrays.
        let vert_len = vert_offset_south_cap + longitudes;
        let mut vs: Vec<Vec3> = Vec::with_capacity(vert_len);
        let mut vts: Vec<Vec2> = Vec::with_capacity(vert_len);
        let mut vns: Vec<Vec3> = Vec::with_capacity(vert_len);

        for _ in 0..vert_len {
            vs.push(Vec3::zero());
            vts.push(Vec2::zero());
            vns.push(Vec3::zero());
        }

        let to_theta = 2.0 * std::f32::consts::PI / longitudes as f32;
        let to_phi = std::f32::consts::PI / latitudes as f32;
        let to_tex_horizontal = 1.0 / longitudes as f32;
        let to_tex_vertical = 1.0 / half_lats as f32;

        let vt_aspect_ratio = match uv_profile {
            CapsuleUvProfile::Aspect => radius / (depth + radius + radius),
            CapsuleUvProfile::Uniform => half_lats as f32 / (ringsp1 + latitudes) as f32,
            CapsuleUvProfile::Fixed => 1.0 / 3.0,
        };
        let vt_aspect_north = 1.0 - vt_aspect_ratio;
        let vt_aspect_south = vt_aspect_ratio;

        let mut theta_cartesian: Vec<Vec2> = Vec::with_capacity(longitudes);
        let mut rho_theta_cartesian: Vec<Vec2> = Vec::with_capacity(longitudes);
        let mut s_texture_cache: Vec<f32> = Vec::with_capacity(lonsp1);

        for _ in 0..longitudes {
            theta_cartesian.push(Vec2::zero());
            rho_theta_cartesian.push(Vec2::zero());
        }
        for _ in 0..lonsp1 {
            s_texture_cache.push(0.0);
        }

        for j in 0..longitudes {
            let jf = j as f32;
            let s_texture_polar = 1.0 - ((jf + 0.5) * to_tex_horizontal);
            let theta = jf * to_theta;

            let cos_theta = theta.cos();
            let sin_theta = theta.sin();

            theta_cartesian[j] = Vec2::new(cos_theta, sin_theta);
            rho_theta_cartesian[j] = Vec2::new(radius * cos_theta, radius * sin_theta);

            // North.
            vs[j] = Vec3::new(0.0, summit, 0.0);
            vts[j] = Vec2::new(s_texture_polar, 1.0);
            vns[j] = Vec3::new(0.0, 1.0, 0.0);

            // South.
            let idx = vert_offset_south_cap + j;
            vs[idx] = Vec3::new(0.0, -summit, 0.0);
            vts[idx] = Vec2::new(s_texture_polar, 0.0);
            vns[idx] = Vec3::new(0.0, -1.0, 0.0);
        }

        // Equatorial vertices.
        for j in 0..lonsp1 {
            let s_texture = 1.0 - j as f32 * to_tex_horizontal;
            s_texture_cache[j] = s_texture;

            // Wrap to first element upon reaching last.
            let j_mod = j % longitudes;
            let tc = theta_cartesian[j_mod];
            let rtc = rho_theta_cartesian[j_mod];

            // North equator.
            let idxn = vert_offset_north_equator + j;
            vs[idxn] = Vec3::new(rtc.x, half_depth, -rtc.y);
            vts[idxn] = Vec2::new(s_texture, vt_aspect_north);
            vns[idxn] = Vec3::new(tc.x, 0.0, -tc.y);

            // South equator.
            let idxs = vert_offset_south_equator + j;
            vs[idxs] = Vec3::new(rtc.x, -half_depth, -rtc.y);
            vts[idxs] = Vec2::new(s_texture, vt_aspect_south);
            vns[idxs] = Vec3::new(tc.x, 0.0, -tc.y);
        }

        // Hemisphere vertices.
        for i in 0..half_latsn1 {
            let ip1f = i as f32 + 1.0;
            let phi = ip1f * to_phi;

            // For coordinates.
            let cos_phi_south = phi.cos();
            let sin_phi_south = phi.sin();

            // Symmetrical hemispheres mean cosine and sine only needs
            // to be calculated once.
            let cos_phi_north = sin_phi_south;
            let sin_phi_north = -cos_phi_south;

            let rho_cos_phi_north = radius * cos_phi_north;
            let rho_sin_phi_north = radius * sin_phi_north;
            let z_offset_north = half_depth - rho_sin_phi_north;

            let rho_cos_phi_south = radius * cos_phi_south;
            let rho_sin_phi_south = radius * sin_phi_south;
            let z_offset_sout = -half_depth - rho_sin_phi_south;

            // For texture coordinates.
            let t_tex_fac = ip1f * to_tex_vertical;
            let cmpl_tex_fac = 1.0 - t_tex_fac;
            let t_tex_north = cmpl_tex_fac + vt_aspect_north * t_tex_fac;
            let t_tex_south = cmpl_tex_fac * vt_aspect_south;

            let i_lonsp1 = i * lonsp1;
            let vert_curr_lat_north = vert_offset_north_hemi + i_lonsp1;
            let vert_curr_lat_south = vert_offset_south_hemi + i_lonsp1;

            for j in 0..lonsp1 {
                let j_mod = j % longitudes;

                let s_texture = s_texture_cache[j];
                let tc = theta_cartesian[j_mod];

                // North hemisphere.
                let idxn = vert_curr_lat_north + j;
                vs[idxn] = Vec3::new(
                    rho_cos_phi_north * tc.x,
                    z_offset_north,
                    -rho_cos_phi_north * tc.y,
                );
                vts[idxn] = Vec2::new(s_texture, t_tex_north);
                vns[idxn] = Vec3::new(cos_phi_north * tc.x, -sin_phi_north, -cos_phi_north * tc.y);

                // South hemisphere.
                let idxs = vert_curr_lat_south + j;
                vs[idxs] = Vec3::new(
                    rho_cos_phi_south * tc.x,
                    z_offset_sout,
                    -rho_cos_phi_south * tc.y,
                );
                vts[idxs] = Vec2::new(s_texture, t_tex_south);
                vns[idxs] = Vec3::new(cos_phi_south * tc.x, -sin_phi_south, -cos_phi_south * tc.y);
            }
        }

        // Cylinder vertices.
        if calc_middle {
            // Exclude both origin and destination edges
            // (North and South equators) from the interpolation.
            let to_fac = 1.0 / ringsp1 as f32;
            let mut idx_cyl_lat = vert_offset_cylinder;

            for h in 1..ringsp1 {
                let fac = h as f32 * to_fac;
                let cmpl_fac = 1.0 - fac;
                let t_texture = cmpl_fac * vt_aspect_north + fac * vt_aspect_south;
                let z = half_depth - depth * fac;

                for j in 0..lonsp1 {
                    let j_mod = j % longitudes;
                    let tc = theta_cartesian[j_mod];
                    let rtc = rho_theta_cartesian[j_mod];
                    let s_texture = s_texture_cache[j];

                    vs[idx_cyl_lat] = Vec3::new(rtc.x, z, -rtc.y);
                    vts[idx_cyl_lat] = Vec2::new(s_texture, t_texture);
                    vns[idx_cyl_lat] = Vec3::new(tc.x, 0.0, -tc.y);

                    idx_cyl_lat += 1;
                }
            }
        }

        // Triangle indices.

        // Stride is 3 for polar triangles;
        // stride is 6 for two triangles forming a quad.
        let lons3 = longitudes * 3;
        let lons6 = longitudes * 6;
        let hemi_lons = half_latsn1 * lons6;

        let tri_offset_north_hemi = lons3;
        let tri_offset_cylinder = tri_offset_north_hemi + hemi_lons;
        let tri_offset_south_hemi = tri_offset_cylinder + ringsp1 * lons6;
        let tri_offset_south_cap = tri_offset_south_hemi + hemi_lons;

        let fs_len = tri_offset_south_cap + lons3;
        let mut tris: Vec<u32> = Vec::with_capacity(fs_len);

        for _ in 0..fs_len {
            tris.push(0);
        }

        // Polar caps.
        let mut i = 0;
        let mut k = 0;
        let mut m = tri_offset_south_cap;
        while i < longitudes {
            // North.
            tris[k] = i as u32;
            tris[k + 1] = (vert_offset_north_hemi + i) as u32;
            tris[k + 2] = (vert_offset_north_hemi + i + 1) as u32;

            // South.
            tris[m] = (vert_offset_south_cap + i) as u32;
            tris[m + 1] = (vert_offset_south_polar + i + 1) as u32;
            tris[m + 2] = (vert_offset_south_polar + i) as u32;

            i += 1;
            k += 3;
            m += 3;
        }

        // Hemispheres.

        let mut i = 0;
        let mut k = tri_offset_north_hemi;
        let mut m = tri_offset_south_hemi;

        while i < half_latsn1 {
            let i_lonsp1 = i * lonsp1;

            let vert_curr_lat_north = vert_offset_north_hemi + i_lonsp1;
            let vert_next_lat_north = vert_curr_lat_north + lonsp1;

            let vert_curr_lat_south = vert_offset_south_equator + i_lonsp1;
            let vert_next_lat_south = vert_curr_lat_south + lonsp1;

            let mut j = 0;
            while j < longitudes {
                // North.
                let north00 = vert_curr_lat_north + j;
                let north01 = vert_next_lat_north + j;
                let north11 = vert_next_lat_north + j + 1;
                let north10 = vert_curr_lat_north + j + 1;

                tris[k] = north00 as u32;
                tris[k + 1] = north11 as u32;
                tris[k + 2] = north10 as u32;

                tris[k + 3] = north00 as u32;
                tris[k + 4] = north01 as u32;
                tris[k + 5] = north11 as u32;

                // South.
                let south00 = vert_curr_lat_south + j;
                let south01 = vert_next_lat_south + j;
                let south11 = vert_next_lat_south + j + 1;
                let south10 = vert_curr_lat_south + j + 1;

                tris[m] = south00 as u32;
                tris[m + 1] = south11 as u32;
                tris[m + 2] = south10 as u32;

                tris[m + 3] = south00 as u32;
                tris[m + 4] = south01 as u32;
                tris[m + 5] = south11 as u32;

                j += 1;
                k += 6;
                m += 6;
            }

            i += 1;
        }

        // Cylinder.
        let mut i = 0;
        let mut k = tri_offset_cylinder;

        while i < ringsp1 {
            let vert_curr_lat = vert_offset_north_equator + i * lonsp1;
            let vert_next_lat = vert_curr_lat + lonsp1;

            let mut j = 0;
            while j < longitudes {
                let cy00 = vert_curr_lat + j;
                let cy01 = vert_next_lat + j;
                let cy11 = vert_next_lat + j + 1;
                let cy10 = vert_curr_lat + j + 1;

                tris[k] = cy00 as u32;
                tris[k + 1] = cy11 as u32;
                tris[k + 2] = cy10 as u32;

                tris[k + 3] = cy00 as u32;
                tris[k + 4] = cy01 as u32;
                tris[k + 5] = cy11 as u32;

                j += 1;
                k += 6;
            }

            i += 1;
        }

        let vs: Vec<[f32; 3]> = vs.into_iter().map(Into::into).collect();
        let vns: Vec<[f32; 3]> = vns.into_iter().map(Into::into).collect();
        let vts: Vec<[f32; 2]> = vts.into_iter().map(Into::into).collect();

        assert_eq!(vs.len(), vert_len);
        assert_eq!(tris.len(), fs_len);

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, vs);
        mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, vns);
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, vts);
        mesh.set_indices(Some(Indices::U32(tris)));
        mesh
    }
}
