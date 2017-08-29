
extern crate gltf;

use std::io::Write;

fn main() {
    let path = "test-data/Avocado.gltf";
    let gltf = gltf::Import::from_path(path).sync().unwrap();
    let mesh = gltf.meshes().nth(0).unwrap();
    let primitive = mesh.primitives().nth(0).unwrap();
    let positions: Vec<[f32; 3]> = primitive.positions().unwrap().collect();
    let normals: Vec<[f32; 3]> = primitive.normals().unwrap().collect();
    let mut tex_coords: Vec<[f32; 2]> = vec![];
    let mut indices: Vec<u16> = vec![];
    match primitive.tex_coords(0).unwrap() {
        gltf::mesh::TexCoords::F32(iter) => tex_coords.extend(iter),
        _ => unreachable!(),
    }
    match primitive.indices().unwrap() {
        gltf::mesh::Indices::U16(iter) => indices.extend(iter),
        _ => unreachable!(),
    }

    let file = std::fs::File::create("Avocado.obj").unwrap();
    let mut writer = std::io::BufWriter::new(file);
    for position in &positions {
        writeln!(writer, "v {} {} {}", position[0], position[1], position[2]);
    }
    for normal in &normals {
        writeln!(writer, "vn {} {} {}", normal[0], normal[1], normal[2]);
    }
    for tex_coord in &tex_coords {
        writeln!(writer, "vt {} {}", tex_coord[0], tex_coord[1]);
    }
    let mut i = indices.iter();
    while let (Some(v0), Some(v1), Some(v2)) = (i.next(), i.next(), i.next()) {
        writeln!(
            writer,
            "f {}/{}/{} {}/{}/{} {}/{}/{}",
            1 + v0, 1 + v0, 1 + v0,
            1 + v1, 1 + v1, 1 + v1,
            1 + v2, 1 + v2, 1 + v2,
        );
    }
}
