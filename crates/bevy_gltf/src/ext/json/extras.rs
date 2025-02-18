use crate::GltfExtras;

pub trait ExtrasExt {
    fn as_gltf_extras(&self) -> Option<GltfExtras>;
}

impl ExtrasExt for gltf::json::Extras {
    fn as_gltf_extras(&self) -> Option<GltfExtras> {
        self.as_ref().map(|extras| GltfExtras {
            value: extras.get().to_string(),
        })
    }
}
