//! Extentions traits for [`gltf::json`] types

pub mod extras;
#[cfg(any(
    feature = "pbr_anisotropy_texture",
    feature = "pbr_multi_layer_material_textures",
    feature = "pbr_specular_textures"
))]
pub mod info;
