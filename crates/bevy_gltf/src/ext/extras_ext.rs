use crate::GltfExtras;

/// [`Extras`](gltf::json::Extras) extension
pub trait ExtrasExt {
    fn get(&self) -> Option<GltfExtras>;
}

impl ExtrasExt for gltf::json::Extras {
    fn get(&self) -> Option<GltfExtras> {
        self.as_ref().map(|extras| GltfExtras {
            value: extras.get().to_string(),
        })
    }
}
