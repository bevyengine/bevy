use std::{fs::File, io::Read};

use bevy_mikktspace::{generate_tangents, Geometry};
use bytemuck::{bytes_of_mut, cast_slice_mut, Pod, Zeroable};

struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

#[derive(Pod, Zeroable, Default, Debug, Clone, Copy)]
#[repr(C)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    texture_coords: [f32; 2],
    tangent: [f32; 3],
}

fn vertex(mesh: &Mesh, face: usize, vert: usize) -> &Vertex {
    let index = mesh.indices[(face * 3) + vert];
    &mesh.vertices[index as usize]
}

impl Geometry for Mesh {
    fn num_faces(&self) -> usize {
        self.indices.len() / 3
    }

    fn num_vertices_of_face(&self, _face: usize) -> usize {
        3
    }

    fn position(&self, face: usize, vert: usize) -> [f32; 3] {
        vertex(self, face, vert).position
    }

    fn normal(&self, face: usize, vert: usize) -> [f32; 3] {
        vertex(self, face, vert).normal
    }

    fn tex_coord(&self, face: usize, vert: usize) -> [f32; 2] {
        vertex(self, face, vert).texture_coords
    }

    fn set_tangent(
        &mut self,
        tangent: [f32; 3],
        _bi_tangent: [f32; 3],
        _mag_s: f32,
        _mag_t: f32,
        _bi_tangent_preserves_orientation: bool,
        face: usize,
        vert: usize,
    ) {
        let index = self.indices[(face * 3) + vert];
        self.vertices[index as usize].tangent = tangent;
    }
}

fn load_mesh(path: &str) -> Mesh {
    println!("loading mesh data");
    let mut mesh = Mesh {
        vertices: Vec::new(),
        indices: Vec::new(),
    };

    // Open the file
    let mut file = File::open(path).unwrap();

    // Read the vertices
    let mut vertices_len = 0u32;
    file.read_exact(bytes_of_mut(&mut vertices_len)).unwrap();

    mesh.vertices = vec![Vertex::default(); vertices_len as usize];
    file.read_exact(cast_slice_mut(&mut mesh.vertices)).unwrap();

    // Read the indices
    let mut indices_len = 0u32;
    file.read_exact(bytes_of_mut(&mut indices_len)).unwrap();

    mesh.indices = vec![0; indices_len as usize];
    file.read_exact(cast_slice_mut(&mut mesh.indices)).unwrap();

    println!("read {} vertices, {} indices", vertices_len, indices_len);

    mesh
}

fn match_at(path: &str) {
    // Load the mesh
    let mut mesh = load_mesh(path);

    // Store the original reference values, and zero tangents just in case
    let original = mesh.vertices.clone();
    for vertex in &mut mesh.vertices {
        vertex.tangent = [0.0, 0.0, 0.0];
    }

    // Perform tangent generation
    println!("generating tangents");
    generate_tangents(&mut mesh);

    // Match against original
    assert!(!original.is_empty());
    assert_eq!(original.len(), mesh.vertices.len());
    println!("validating {} tangents", original.len());
    for (i, original) in original.iter().enumerate() {
        assert_eq!(original.tangent, mesh.vertices[i].tangent);
    }
}

#[test]
fn match_cube() {
    match_at("data/cube.bin");
}

#[test]
fn match_suzanne_flat() {
    match_at("data/suzanne_flat_tris.bin");
}

#[test]
fn match_suzanne_smooth() {
    match_at("data/suzanne_smooth_tris.bin");
}

#[test]
fn match_suzanne_bad() {
    // This model intentionally contains bad faces that can't have tangents generated for it
    match_at("data/suzanne_bad.bin");
}
