use std::convert::From;
use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
#[derive(Clone, Copy, AsBytes, FromBytes)]
pub struct Vertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
}

impl From<([f32; 4], [f32; 4])> for Vertex {
    fn from((position, normal): ([f32; 4], [f32; 4])) -> Self {
        Vertex {
            position: position,
            normal: normal,
        }
    }
}

impl From<([f32; 3], [f32; 3])> for Vertex {
    fn from((position, normal): ([f32; 3], [f32; 3])) -> Self {
        Vertex {
            position: [
                position[0] as f32,
                position[1] as f32,
                position[2] as f32,
                1.0,
            ],
            normal: [normal[0] as f32, normal[1] as f32, normal[2] as f32, 0.0],
        }
    }
}

impl From<([i8; 4], [i8; 4])> for Vertex {
    fn from((position, normal): ([i8; 4], [i8; 4])) -> Self {
        Vertex {
            position: [
                position[0] as f32,
                position[1] as f32,
                position[2] as f32,
                position[3] as f32,
            ],
            normal: [
                normal[0] as f32,
                normal[1] as f32,
                normal[2] as f32,
                normal[3] as f32,
            ],
        }
    }
}

impl From<([i8; 3], [i8; 3])> for Vertex {
    fn from((position, normal): ([i8; 3], [i8; 3])) -> Self {
        Vertex {
            position: [
                position[0] as f32,
                position[1] as f32,
                position[2] as f32,
                1.0,
            ],
            normal: [normal[0] as f32, normal[1] as f32, normal[2] as f32, 0.0],
        }
    }
}
