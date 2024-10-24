#[cfg(feature = "bevy_animation")]
mod animation_ext;
mod extras_ext;
mod gltf_ext;
#[cfg(any(
    feature = "pbr_anisotropy_texture",
    feature = "pbr_multi_layer_material_textures"
))]
mod json_texture_info_ext;
mod material_ext;
mod mesh_ext;
mod mode_ext;
mod node_ext;
mod primitive_ext;
mod scene_ext;
mod skin_ext;
mod texture_ext;
mod texture_info_ext;
mod texture_transform_ext;
mod wrapping_mode_ext;

pub use self::{
    extras_ext::ExtrasExt, gltf_ext::GltfExt, material_ext::MaterialExt, mesh_ext::MeshExt,
    mode_ext::ModeExt, node_ext::NodeExt, primitive_ext::PrimitiveExt, scene_ext::SceneExt,
    skin_ext::SkinExt, texture_ext::TextureExt, texture_info_ext::TextureInfoExt,
    texture_transform_ext::TextureTransformExt, wrapping_mode_ext::WrappingModeExt,
};

#[cfg(any(
    feature = "pbr_anisotropy_texture",
    feature = "pbr_multi_layer_material_textures"
))]
pub use self::json_texture_info_ext::JsonTextureInfoExt;
