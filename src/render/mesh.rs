use crate::{asset::Asset, math::*, render::Vertex};
use wgpu::{Buffer, Device};
use zerocopy::AsBytes;

// TODO: this is pretty dirty. work out a cleaner way to distinguish between 3d and 2d meshes
pub struct Mesh2d;

pub enum MeshType {
    Cube,
    Plane {
        size: f32,
    },
    Quad {
        north_west: Vec2,
        north_east: Vec2,
        south_west: Vec2,
        south_east: Vec2,
    },
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub vertex_buffer: Option<Buffer>,
    pub index_buffer: Option<Buffer>,
}

impl Mesh {
    pub fn setup_buffers(&mut self, device: &Device) {
        if let None = self.vertex_buffer {
            self.vertex_buffer = Some(
                device.create_buffer_with_data(self.vertices.as_bytes(), wgpu::BufferUsage::VERTEX),
            );
        }

        if let None = self.index_buffer {
            self.index_buffer = Some(
                device.create_buffer_with_data(self.indices.as_bytes(), wgpu::BufferUsage::INDEX),
            );
        }
    }
}

impl Asset<MeshType> for Mesh {
    fn load(descriptor: MeshType) -> Self {
        let (vertices, indices) = match descriptor {
            MeshType::Cube => create_cube(),
            MeshType::Plane { size } => create_plane(size),
            MeshType::Quad {
                north_west,
                north_east,
                south_west,
                south_east,
            } => create_quad(north_west, north_east, south_west, south_east),
        };

        Mesh {
            vertices,
            indices,
            vertex_buffer: None,
            index_buffer: None,
        }
    }
}

pub fn create_quad(
    north_west: Vec2,
    north_east: Vec2,
    south_west: Vec2,
    south_east: Vec2,
) -> (Vec<Vertex>, Vec<u16>) {
    let vertex_data = [
        Vertex::from(([south_west.x(), south_west.y(), 0.0], [0.0, 0.0, 1.0])),
        Vertex::from(([north_west.x(), north_west.y(), 0.0], [0.0, 0.0, 1.0])),
        Vertex::from(([north_east.x(), north_east.y(), 0.0], [0.0, 0.0, 1.0])),
        Vertex::from(([south_east.x(), south_east.y(), 0.0], [0.0, 0.0, 1.0])),
    ];

    let index_data: &[u16] = &[0, 1, 2, 0, 2, 3];
    return (vertex_data.to_vec(), index_data.to_vec());
}

pub fn create_cube() -> (Vec<Vertex>, Vec<u16>) {
    let vertex_data = [
        // top (0, 0, 1)
        Vertex::from(([-1, -1, 1], [0, 0, 1])),
        Vertex::from(([1, -1, 1], [0, 0, 1])),
        Vertex::from(([1, 1, 1], [0, 0, 1])),
        Vertex::from(([-1, 1, 1], [0, 0, 1])),
        // bottom (0, 0, -1)
        Vertex::from(([-1, 1, -1], [0, 0, -1])),
        Vertex::from(([1, 1, -1], [0, 0, -1])),
        Vertex::from(([1, -1, -1], [0, 0, -1])),
        Vertex::from(([-1, -1, -1], [0, 0, -1])),
        // right (1, 0, 0)
        Vertex::from(([1, -1, -1], [1, 0, 0])),
        Vertex::from(([1, 1, -1], [1, 0, 0])),
        Vertex::from(([1, 1, 1], [1, 0, 0])),
        Vertex::from(([1, -1, 1], [1, 0, 0])),
        // left (-1, 0, 0)
        Vertex::from(([-1, -1, 1], [-1, 0, 0])),
        Vertex::from(([-1, 1, 1], [-1, 0, 0])),
        Vertex::from(([-1, 1, -1], [-1, 0, 0])),
        Vertex::from(([-1, -1, -1], [-1, 0, 0])),
        // front (0, 1, 0)
        Vertex::from(([1, 1, -1], [0, 1, 0])),
        Vertex::from(([-1, 1, -1], [0, 1, 0])),
        Vertex::from(([-1, 1, 1], [0, 1, 0])),
        Vertex::from(([1, 1, 1], [0, 1, 0])),
        // back (0, -1, 0)
        Vertex::from(([1, -1, 1], [0, -1, 0])),
        Vertex::from(([-1, -1, 1], [0, -1, 0])),
        Vertex::from(([-1, -1, -1], [0, -1, 0])),
        Vertex::from(([1, -1, -1], [0, -1, 0])),
    ];

    let index_data: &[u16] = &[
        0, 1, 2, 2, 3, 0, // top
        4, 5, 6, 6, 7, 4, // bottom
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // front
        20, 21, 22, 22, 23, 20, // back
    ];

    (vertex_data.to_vec(), index_data.to_vec())
}

pub fn create_plane(size: f32) -> (Vec<Vertex>, Vec<u16>) {
    let vertex_data = [
        Vertex::from(([size, -size, 0.0], [0.0, 0.0, 1.0])),
        Vertex::from(([size, size, 0.0], [0.0, 0.0, 1.0])),
        Vertex::from(([-size, -size, 0.0], [0.0, 0.0, 1.0])),
        Vertex::from(([-size, size, 0.0], [0.0, 0.0, 1.0])),
    ];

    // TODO: make sure this order is correct
    let index_data: &[u16] = &[0, 1, 2, 2, 1, 3];

    (vertex_data.to_vec(), index_data.to_vec())
}
