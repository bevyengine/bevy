use bevy_render::{mesh::*, pipeline::PrimitiveTopology};

pub struct CoordinateGizmo {
    pub size: f32,
}

impl CoordinateGizmo {
    pub fn new(size: f32) -> CoordinateGizmo {
        CoordinateGizmo { size }
    }
}

impl Default for CoordinateGizmo {
    pub fn default() -> Self {
        CoordinateGizmo { size: 1.0 }
    }
}

impl From<CoordinateGizmo> for Mesh {
    pub fn from(shape: CoordinateGizmo) -> Self {
        let mut mesh = Mesh::new(PrimitiveTopology::LineList);
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [shape.size, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [0.0, shape.size, 0.0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, shape.size],
        ];
        mesh.set_attribute(
            Mesh::ATTRIBUTE_POSITION,
            VertexAttributeValues::Float3(vertices),
        );
        let colors = vec![
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0, 1.0],
            [0.0, 0.0, 1.0, 1.0],
        ];
        mesh.set_attribute("Vertex_Color", VertexAttributeValues::Float4(colors));
        mesh
    }
}
