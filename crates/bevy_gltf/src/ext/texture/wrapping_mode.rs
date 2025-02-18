use bevy_image::ImageAddressMode;
use gltf::texture::WrappingMode;

pub trait WrappingModeExt {
    fn address_mode(&self) -> ImageAddressMode;
}

impl WrappingModeExt for WrappingMode {
    /// Maps the texture address mode from glTF to wgpu.
    fn address_mode(&self) -> ImageAddressMode {
        match self {
            WrappingMode::ClampToEdge => ImageAddressMode::ClampToEdge,
            WrappingMode::Repeat => ImageAddressMode::Repeat,
            WrappingMode::MirroredRepeat => ImageAddressMode::MirrorRepeat,
        }
    }
}
