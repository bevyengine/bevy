use gltf::json::Extras;

use crate::GltfExtras;

pub(crate) fn as_gltf_extras(extras: &Extras) -> Option<GltfExtras> {
    extras.as_ref().map(|extras| GltfExtras {
        value: extras.get().to_string(),
    })
}
