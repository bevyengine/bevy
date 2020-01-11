use std::boxed::Box;
use std::error::Error;
use std::{fs, io};

// use crate::render::Mesh;

pub fn load_gltf(path: &str) -> Result<(), Box<dyn Error>> {
    println!("{}", path);
    let file = fs::File::open(&path)?;
    let reader = io::BufReader::new(file);
    let gltf = gltf::Gltf::from_reader(reader)?;
    for scene in gltf.scenes() {
        for _mesh in scene.nodes() {}
    }

    Ok(())
}
