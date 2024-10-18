use gltf::json;

use crate::GltfExtras;

pub fn get_gltf_extras(extras: &json::Extras) -> Option<GltfExtras> {
    extras.as_ref().map(|extras| GltfExtras {
        value: extras.get().to_string(),
    })
}
