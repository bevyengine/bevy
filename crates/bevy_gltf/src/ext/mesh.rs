pub mod mode;

use gltf::{Mesh, Primitive};

pub trait MeshExt {
    fn primitive_name(&self, primitive: &Primitive) -> String;
}

impl MeshExt for Mesh<'_> {
    fn primitive_name(&self, primitive: &Primitive) -> String {
        let mesh_name = self.name().unwrap_or("Mesh");
        if self.primitives().len() > 1 {
            format!("{}.{}", mesh_name, primitive.index())
        } else {
            mesh_name.to_string()
        }
    }
}
