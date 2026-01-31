use bevy_asset::{Asset, Handle};
use bevy_color::{Color, LinearRgba};
use bevy_image::Image;
use bevy_material::{AlphaMode, UvChannel};
use bevy_math::Affine2;
use bevy_reflect::TypePath;
use wgpu_types::Face;

/// Data to build a Gltf Material
#[derive(Asset, Debug, TypePath)]
pub struct GltfMaterial {
    // TODO: copy comments from standard material?
    /// A
    pub base_color: Color,
    /// A
    pub base_color_channel: UvChannel,
    /// A
    pub base_color_texture: Option<Handle<Image>>,
    /// A
    pub emissive: LinearRgba,
    /// A
    pub emissive_channel: UvChannel,
    /// A
    pub emissive_texture: Option<Handle<Image>>,
    /// A
    pub perceptual_roughness: f32,
    /// A
    pub metallic: f32,
    /// A
    pub metallic_roughness_channel: UvChannel,
    /// A
    pub metallic_roughness_texture: Option<Handle<Image>>,
    /// A
    pub reflectance: f32,
    /// A
    #[cfg(feature = "pbr_specular_textures")]
    pub specular_channel: UvChannel,
    /// A
    #[cfg(feature = "pbr_specular_textures")]
    pub specular_texture: Option<Handle<Image>>,
    /// A
    pub specular_tint: Color,
    /// A
    #[cfg(feature = "pbr_specular_textures")]
    pub specular_tint_channel: UvChannel,
    /// A
    #[cfg(feature = "pbr_specular_textures")]
    pub specular_tint_texture: Option<Handle<Image>>,
    /// A
    pub specular_transmission: f32,
    /// A
    #[cfg(feature = "pbr_transmission_textures")]
    pub specular_transmission_channel: UvChannel,
    /// A
    #[cfg(feature = "pbr_transmission_textures")]
    pub specular_transmission_texture: Option<Handle<Image>>,
    /// A
    pub thickness: f32,
    #[cfg(feature = "pbr_transmission_textures")]
    /// A
    pub thickness_channel: UvChannel,
    /// A
    #[cfg(feature = "pbr_transmission_textures")]
    pub thickness_texture: Option<Handle<Image>>,
    /// A
    pub ior: f32,
    /// A
    pub attenuation_distance: f32,
    /// A
    pub attenuation_color: Color,
    /// A
    pub normal_map_channel: UvChannel,
    /// A
    pub normal_map_texture: Option<Handle<Image>>,
    /// A
    pub occlusion_channel: UvChannel,
    /// A
    pub occlusion_texture: Option<Handle<Image>>,
    /// A
    pub clearcoat: f32,
    /// A
    pub clearcoat_perceptual_roughness: f32,
    /// A
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_channel: UvChannel,
    /// A
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_texture: Option<Handle<Image>>,
    /// A
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_roughness_channel: UvChannel,
    /// A
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_roughness_texture: Option<Handle<Image>>,
    /// A
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_normal_channel: UvChannel,
    /// A
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    /// A
    pub clearcoat_normal_texture: Option<Handle<Image>>,
    /// A
    pub anisotropy_strength: f32,
    /// A
    pub anisotropy_rotation: f32,
    /// A
    #[cfg(feature = "pbr_anisotropy_texture")]
    pub anisotropy_channel: UvChannel,
    /// A
    #[cfg(feature = "pbr_anisotropy_texture")]
    pub anisotropy_texture: Option<Handle<Image>>,
    /// A
    pub double_sided: bool,
    /// A
    pub cull_mode: Option<Face>,
    /// A
    pub unlit: bool,
    /// A
    pub alpha_mode: AlphaMode,
    /// A
    pub uv_transform: Affine2,
}

impl Default for GltfMaterial {
    fn default() -> Self {
        GltfMaterial {
            // White because it gets multiplied with texture values if someone uses
            // a texture.
            base_color: Color::WHITE,
            base_color_channel: UvChannel::Uv0,
            base_color_texture: None,
            emissive: LinearRgba::BLACK,
            emissive_channel: UvChannel::Uv0,
            emissive_texture: None,
            // Matches Blender's default roughness.
            perceptual_roughness: 0.5,
            // Metallic should generally be set to 0.0 or 1.0.
            metallic: 0.0,
            metallic_roughness_channel: UvChannel::Uv0,
            metallic_roughness_texture: None,
            // Minimum real-world reflectance is 2%, most materials between 2-5%
            // Expressed in a linear scale and equivalent to 4% reflectance see
            // <https://google.github.io/filament/Material%20Properties.pdf>
            reflectance: 0.5,
            specular_transmission: 0.0,
            #[cfg(feature = "pbr_transmission_textures")]
            specular_transmission_channel: UvChannel::Uv0,
            #[cfg(feature = "pbr_transmission_textures")]
            specular_transmission_texture: None,
            thickness: 0.0,
            #[cfg(feature = "pbr_transmission_textures")]
            thickness_channel: UvChannel::Uv0,
            #[cfg(feature = "pbr_transmission_textures")]
            thickness_texture: None,
            ior: 1.5,
            attenuation_color: Color::WHITE,
            attenuation_distance: f32::INFINITY,
            occlusion_channel: UvChannel::Uv0,
            occlusion_texture: None,
            normal_map_channel: UvChannel::Uv0,
            normal_map_texture: None,
            #[cfg(feature = "pbr_specular_textures")]
            specular_channel: UvChannel::Uv0,
            #[cfg(feature = "pbr_specular_textures")]
            specular_texture: None,
            specular_tint: Color::WHITE,
            #[cfg(feature = "pbr_specular_textures")]
            specular_tint_channel: UvChannel::Uv0,
            #[cfg(feature = "pbr_specular_textures")]
            specular_tint_texture: None,
            clearcoat: 0.0,
            clearcoat_perceptual_roughness: 0.5,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_channel: UvChannel::Uv0,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_texture: None,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_roughness_channel: UvChannel::Uv0,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_roughness_texture: None,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_normal_channel: UvChannel::Uv0,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_normal_texture: None,
            anisotropy_strength: 0.0,
            anisotropy_rotation: 0.0,
            #[cfg(feature = "pbr_anisotropy_texture")]
            anisotropy_channel: UvChannel::Uv0,
            #[cfg(feature = "pbr_anisotropy_texture")]
            anisotropy_texture: None,
            double_sided: false,
            cull_mode: Some(Face::Back),
            unlit: false,
            alpha_mode: AlphaMode::Opaque,
            uv_transform: Affine2::IDENTITY,
        }
    }
}
