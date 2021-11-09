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

/// A rectangle on the XY plane centered at the origin.
#[derive(Debug, Copy, Clone)]
pub struct Quad {
    /// Full width and height of the rectangle.
    pub size: Vec2,
    /// Flips the texture coords of the resulting vertices.
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

/// A square on the XZ plane centered at the origin.
#[derive(Debug, Copy, Clone)]
pub struct Plane {
    /// The total side length of the square.
    pub size: f32,
}

impl Default for Plane {
    fn default() -> Self {
        Plane { size: 1.0 }
    }
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

mod capsule;
mod icosphere;
mod torus;
mod uvsphere;

pub use capsule::{Capsule, CapsuleUvProfile};
pub use icosphere::Icosphere;
pub use torus::Torus;
pub use uvsphere::UVSphere;
use wgpu::PrimitiveTopology;
