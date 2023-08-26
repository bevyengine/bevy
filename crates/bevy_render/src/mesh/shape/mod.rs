use super::{Indices, Mesh};
use bevy_math::*;

#[derive(Debug, Copy, Clone)]
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

/// An axis-aligned box defined by its minimum and maximum point.
#[derive(Debug, Copy, Clone)]
pub struct Box {
    pub min_x: f32,
    pub max_x: f32,

    pub min_y: f32,
    pub max_y: f32,

    pub min_z: f32,
    pub max_z: f32,
}

impl Box {
    /// Creates a new box centered at the origin with the supplied side lengths.
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

    /// Creates a new box given the coordinates of two opposing corners.
    pub fn from_corners(a: Vec3, b: Vec3) -> Box {
        let max = a.max(b);
        let min = a.min(b);
        Box {
            max_x: max.x,
            min_x: min.x,
            max_y: max.y,
            min_y: min.y,
            max_z: max.z,
            min_z: min.z,
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
        // suppose Y-up right hand, and camera look from +z to -z
        let vertices = &[
            // Front
            ([sp.min_x, sp.min_y, sp.max_z], [0., 0., 1.0], [0., 0.]),
            ([sp.max_x, sp.min_y, sp.max_z], [0., 0., 1.0], [1.0, 0.]),
            ([sp.max_x, sp.max_y, sp.max_z], [0., 0., 1.0], [1.0, 1.0]),
            ([sp.min_x, sp.max_y, sp.max_z], [0., 0., 1.0], [0., 1.0]),
            // Back
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
            // Top
            ([sp.max_x, sp.max_y, sp.min_z], [0., 1.0, 0.], [1.0, 0.]),
            ([sp.min_x, sp.max_y, sp.min_z], [0., 1.0, 0.], [0., 0.]),
            ([sp.min_x, sp.max_y, sp.max_z], [0., 1.0, 0.], [0., 1.0]),
            ([sp.max_x, sp.max_y, sp.max_z], [0., 1.0, 0.], [1.0, 1.0]),
            // Bottom
            ([sp.max_x, sp.min_y, sp.max_z], [0., -1.0, 0.], [0., 0.]),
            ([sp.min_x, sp.min_y, sp.max_z], [0., -1.0, 0.], [1.0, 0.]),
            ([sp.min_x, sp.min_y, sp.min_z], [0., -1.0, 0.], [1.0, 1.0]),
            ([sp.max_x, sp.min_y, sp.min_z], [0., -1.0, 0.], [0., 1.0]),
        ];

        let positions: Vec<_> = vertices.iter().map(|(p, _, _)| *p).collect();
        let normals: Vec<_> = vertices.iter().map(|(_, n, _)| *n).collect();
        let uvs: Vec<_> = vertices.iter().map(|(_, _, uv)| *uv).collect();

        let indices = Indices::U32(vec![
            0, 1, 2, 2, 3, 0, // front
            4, 5, 6, 6, 7, 4, // back
            8, 9, 10, 10, 11, 8, // right
            12, 13, 14, 14, 15, 12, // left
            16, 17, 18, 18, 19, 16, // top
            20, 21, 22, 22, 23, 20, // bottom
        ]);

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(indices));
        mesh
    }
}

/// A rectangle on the `XY` plane centered at the origin.
#[derive(Debug, Copy, Clone)]
pub struct Quad {
    /// Full width and height of the rectangle.
    pub size: Vec2,
    /// Horizontally-flip the texture coordinates of the resulting mesh.
    pub flip: bool,
}

impl Default for Quad {
    fn default() -> Self {
        Quad::new(Vec2::ONE)
    }
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

        let (u_left, u_right) = if quad.flip { (1.0, 0.0) } else { (0.0, 1.0) };
        let vertices = [
            ([-extent_x, -extent_y, 0.0], [0.0, 0.0, 1.0], [u_left, 1.0]),
            ([-extent_x, extent_y, 0.0], [0.0, 0.0, 1.0], [u_left, 0.0]),
            ([extent_x, extent_y, 0.0], [0.0, 0.0, 1.0], [u_right, 0.0]),
            ([extent_x, -extent_y, 0.0], [0.0, 0.0, 1.0], [u_right, 1.0]),
        ];

        let indices = Indices::U32(vec![0, 2, 1, 0, 3, 2]);

        let positions: Vec<_> = vertices.iter().map(|(p, _, _)| *p).collect();
        let normals: Vec<_> = vertices.iter().map(|(_, n, _)| *n).collect();
        let uvs: Vec<_> = vertices.iter().map(|(_, _, uv)| *uv).collect();

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_indices(Some(indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh
    }
}

/// A square on the `XZ` plane centered at the origin.
#[derive(Debug, Copy, Clone)]
pub struct Plane {
    /// The total side length of the square.
    pub size: f32,
    /// The number of subdivisions in the mesh.
    ///
    /// 0 - is the original plane geometry, the 4 points in the XZ plane.
    ///
    /// 1 - is split by 1 line in the middle of the plane on both the X axis and the Z axis, resulting in a plane with 4 quads / 8 triangles.
    ///
    /// 2 - is a plane split by 2 lines on both the X and Z axes, subdividing the plane into 3 equal sections along each axis, resulting in a plane with 9 quads / 18 triangles.
    ///
    /// and so on...
    pub subdivisions: u32,
}

impl Default for Plane {
    fn default() -> Self {
        Plane {
            size: 1.0,
            subdivisions: 0,
        }
    }
}

impl Plane {
    /// Creates a new plane centered at the origin with the supplied side length and zero subdivisions.
    pub fn from_size(size: f32) -> Self {
        Self {
            size,
            subdivisions: 0,
        }
    }
}

impl From<Plane> for Mesh {
    fn from(plane: Plane) -> Self {
        // here this is split in the z and x directions if one ever needs asymmetrical subdivision
        // two Plane struct fields would need to be added instead of the single subdivisions field
        let z_vertex_count = plane.subdivisions + 2;
        let x_vertex_count = plane.subdivisions + 2;
        let num_vertices = (z_vertex_count * x_vertex_count) as usize;
        let num_indices = ((z_vertex_count - 1) * (x_vertex_count - 1) * 6) as usize;
        let up = Vec3::Y.to_array();

        let mut positions: Vec<[f32; 3]> = Vec::with_capacity(num_vertices);
        let mut normals: Vec<[f32; 3]> = Vec::with_capacity(num_vertices);
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(num_vertices);
        let mut indices: Vec<u32> = Vec::with_capacity(num_indices);

        for z in 0..z_vertex_count {
            for x in 0..x_vertex_count {
                let tx = x as f32 / (x_vertex_count - 1) as f32;
                let tz = z as f32 / (z_vertex_count - 1) as f32;
                positions.push([(-0.5 + tx) * plane.size, 0.0, (-0.5 + tz) * plane.size]);
                normals.push(up);
                uvs.push([tx, tz]);
            }
        }

        for y in 0..z_vertex_count - 1 {
            for x in 0..x_vertex_count - 1 {
                let quad = y * x_vertex_count + x;
                indices.push(quad + x_vertex_count + 1);
                indices.push(quad + 1);
                indices.push(quad + x_vertex_count);
                indices.push(quad);
                indices.push(quad + x_vertex_count);
                indices.push(quad + 1);
            }
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_indices(Some(Indices::U32(indices)));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh
    }
}

mod capsule;
mod cylinder;
mod icosphere;
mod regular_polygon;
mod torus;
mod uvsphere;

pub use capsule::{Capsule, CapsuleUvProfile};
pub use cylinder::Cylinder;
pub use icosphere::Icosphere;
pub use regular_polygon::{Circle, RegularPolygon};
pub use torus::Torus;
pub use uvsphere::UVSphere;
use wgpu::PrimitiveTopology;
