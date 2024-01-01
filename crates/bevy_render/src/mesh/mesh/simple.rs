//! Simple mesh builder.
//!
//! See [`SimpleMeshBuilder`] for more information.

use crate::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy_math::{Vec2, Vec3};
use bevy_utils::hashbrown::hash_map;
use bevy_utils::HashMap;

/// `SimpleVertex` cannot be hashed because floats are not hashable.
///
/// This struct contains data from `SimpleVertex` with floats bitcasted to u32.
#[derive(Eq, PartialEq, Hash, Debug)]
struct SimpleVertexHashable {
    position_bits: [u32; 3],
    normal_bits: [u32; 3],
    uv_bits: [u32; 2],
}

#[derive(Copy, Clone, Debug)]
pub struct SimpleVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
}

impl SimpleVertex {
    fn to_hashable(self) -> SimpleVertexHashable {
        let SimpleVertex {
            position,
            normal,
            uv,
        } = self;
        SimpleVertexHashable {
            position_bits: position.to_array().map(f32::to_bits),
            normal_bits: normal.to_array().map(f32::to_bits),
            uv_bits: uv.to_array().map(f32::to_bits),
        }
    }
}

/// Simple mesh builder.
///
/// * Positions, normals, and uvs
/// * Triangle list topology
#[derive(Default, Debug)]
pub struct SimpleMeshBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    vertex_to_index: HashMap<SimpleVertexHashable, u32>,

    triangle_indices: Vec<u32>,
}

impl SimpleMeshBuilder {
    /// Add a vertex without attaching it to a triangle. Return the index of the vertex.
    fn add_vertex(&mut self, vertex: SimpleVertex) -> u32 {
        let SimpleVertex {
            position,
            normal,
            uv,
        } = vertex;
        let new_index = self.vertex_to_index.len().try_into().unwrap();
        match self.vertex_to_index.entry(vertex.to_hashable()) {
            hash_map::Entry::Occupied(o) => *o.get(),
            hash_map::Entry::Vacant(v) => {
                self.positions.push([position.x, position.y, position.z]);
                self.normals.push([normal.x, normal.y, normal.z]);
                self.uvs.push([uv.x, uv.y]);
                *v.insert(new_index)
            }
        }
    }

    /// Add triangle.
    pub fn triangle(&mut self, a: SimpleVertex, b: SimpleVertex, c: SimpleVertex) {
        let a = self.add_vertex(a);
        let b = self.add_vertex(b);
        let c = self.add_vertex(c);
        self.triangle_indices.extend([a, b, c]);
    }

    /// Add two triangles forming a quad (a, b, c) and (c, d, a).
    pub fn quad(&mut self, a: SimpleVertex, b: SimpleVertex, c: SimpleVertex, d: SimpleVertex) {
        let a = self.add_vertex(a);
        let b = self.add_vertex(b);
        let c = self.add_vertex(c);
        let d = self.add_vertex(d);
        self.triangle_indices.extend([a, b, c, c, d, a]);
    }

    /// Finish building the mesh.
    pub fn build(self) -> Mesh {
        assert_eq!(self.positions.len(), self.normals.len());
        assert_eq!(self.positions.len(), self.uvs.len());

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.set_indices(Some(Indices::U32(self.triangle_indices)));
        mesh
    }
}
