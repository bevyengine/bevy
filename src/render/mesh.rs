use crate::{asset::Asset, math::*, render::Vertex};

pub enum MeshType {
    Cube,
    Plane {
        size: f32,
    },
    Quad {
        size: Vec2,
    },
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}

impl Asset<MeshType> for Mesh {
    fn load(descriptor: MeshType) -> Self {
        let (vertices, indices) = match descriptor {
            MeshType::Cube => create_cube(),
            MeshType::Plane { size } => create_plane(size),
            MeshType::Quad {
                size
            } => create_quad(size),
        };

        Mesh { vertices, indices }
    }
}

pub fn create_quad_from_vertices(
    north_west: Vec2,
    north_east: Vec2,
    south_west: Vec2,
    south_east: Vec2,
) -> (Vec<Vertex>, Vec<u16>) {
    let vertex_data = [
        Vertex::from((
            [south_west.x(), south_west.y(), 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0],
        )),
        Vertex::from((
            [north_west.x(), north_west.y(), 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0],
        )),
        Vertex::from((
            [north_east.x(), north_east.y(), 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0],
        )),
        Vertex::from((
            [south_east.x(), south_east.y(), 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 1.0],
        )),
    ];

    let index_data: &[u16] = &[0, 2, 1, 0, 3, 2];
    return (vertex_data.to_vec(), index_data.to_vec());
}

pub fn create_quad(dimensions: Vec2) -> (Vec<Vertex>, Vec<u16>) {
    let extent_x = dimensions.x() / 2.0;
    let extent_y = dimensions.y() / 2.0;
    create_quad_from_vertices(
        vec2(-extent_x, extent_y),
        vec2(extent_x, extent_y),
        vec2(-extent_x, -extent_y),
        vec2(extent_x, -extent_y),
    )
}

pub fn create_cube() -> (Vec<Vertex>, Vec<u16>) {
    let vertex_data = [
        // top (0, 0, 1)
        Vertex::from(([-1, -1, 1], [0, 0, 1], [0, 0])),
        Vertex::from(([1, -1, 1], [0, 0, 1], [1, 0])),
        Vertex::from(([1, 1, 1], [0, 0, 1], [1, 1])),
        Vertex::from(([-1, 1, 1], [0, 0, 1], [0, 1])),
        // bottom (0, 0, -1)
        Vertex::from(([-1, 1, -1], [0, 0, -1], [1, 0])),
        Vertex::from(([1, 1, -1], [0, 0, -1], [0, 0])),
        Vertex::from(([1, -1, -1], [0, 0, -1], [0, 1])),
        Vertex::from(([-1, -1, -1], [0, 0, -1], [1, 1])),
        // right (1, 0, 0)
        Vertex::from(([1, -1, -1], [1, 0, 0], [0, 0])),
        Vertex::from(([1, 1, -1], [1, 0, 0], [1, 0])),
        Vertex::from(([1, 1, 1], [1, 0, 0], [1, 1])),
        Vertex::from(([1, -1, 1], [1, 0, 0], [0, 1])),
        // left (-1, 0, 0)
        Vertex::from(([-1, -1, 1], [-1, 0, 0], [1, 0])),
        Vertex::from(([-1, 1, 1], [-1, 0, 0], [0, 0])),
        Vertex::from(([-1, 1, -1], [-1, 0, 0], [0, 1])),
        Vertex::from(([-1, -1, -1], [-1, 0, 0], [1, 1])),
        // front (0, 1, 0)
        Vertex::from(([1, 1, -1], [0, 1, 0], [1, 0])),
        Vertex::from(([-1, 1, -1], [0, 1, 0], [0, 0])),
        Vertex::from(([-1, 1, 1], [0, 1, 0], [0, 1])),
        Vertex::from(([1, 1, 1], [0, 1, 0], [1, 1])),
        // back (0, -1, 0)
        Vertex::from(([1, -1, 1], [0, -1, 0], [0, 0])),
        Vertex::from(([-1, -1, 1], [0, -1, 0], [1, 0])),
        Vertex::from(([-1, -1, -1], [0, -1, 0], [1, 1])),
        Vertex::from(([1, -1, -1], [0, -1, 0], [0, 1])),
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
    create_quad(vec2(size, size))
}
