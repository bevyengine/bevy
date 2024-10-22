use bevy_image::ImageAddressMode;

pub trait WrappingModeExt {
    fn texture_address_mode(&self) -> ImageAddressMode;
}

impl WrappingModeExt for gltf::texture::WrappingMode {
    /// Maps the texture address mode form glTF to wgpu.
    fn texture_address_mode(&self) -> ImageAddressMode {
        match self {
            gltf::texture::WrappingMode::ClampToEdge => ImageAddressMode::ClampToEdge,
            gltf::texture::WrappingMode::Repeat => ImageAddressMode::Repeat,
            gltf::texture::WrappingMode::MirroredRepeat => ImageAddressMode::MirrorRepeat,
        }
    }
}
