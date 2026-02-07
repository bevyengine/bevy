use bevy_asset::{Asset, Handle};
use bevy_color::{Color, LinearRgba};
use bevy_image::Image;
use bevy_material::AlphaMode;
use bevy_math::Affine2;
use bevy_mesh::UvChannel;
use bevy_reflect::TypePath;
use wgpu_types::Face;

/// Data to build a Gltf Material
///
/// See [`StandardMaterial`](https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html) for details
#[derive(Asset, Debug, Clone, TypePath)]
pub struct GltfMaterial {
    /// The color of the surface of the material before lighting.
    pub base_color: Color,

    /// The UV channel to use for the [`GltfMaterial::base_color_texture`].
    pub base_color_channel: UvChannel,

    /// The texture component of the material's color before lighting.
    pub base_color_texture: Option<Handle<Image>>,

    /// Color the material "emits" to the camera.
    pub emissive: LinearRgba,

    /// The UV channel to use for the [`GltfMaterial::emissive_texture`].
    pub emissive_channel: UvChannel,

    /// The emissive map, multiplies pixels with [`GltfMaterial::emissive`]
    /// to get the final "emitting" color of a surface.
    pub emissive_texture: Option<Handle<Image>>,

    /// Linear perceptual roughness.
    pub perceptual_roughness: f32,

    /// How "metallic" the material appears, within `[0.0, 1.0]`.
    pub metallic: f32,

    /// The UV channel to use for the [`GltfMaterial::metallic_roughness_texture`].
    pub metallic_roughness_channel: UvChannel,

    /// Metallic and roughness maps, stored as a single texture.
    pub metallic_roughness_texture: Option<Handle<Image>>,

    /// Specular intensity for non-metals on a linear scale of `[0.0, 1.0]`.
    pub reflectance: f32,

    /// The UV channel to use for the [`GltfMaterial::specular_texture`].
    #[cfg(feature = "pbr_specular_textures")]
    pub specular_channel: UvChannel,

    /// A map that specifies reflectance for non-metallic materials.
    #[cfg(feature = "pbr_specular_textures")]
    pub specular_texture: Option<Handle<Image>>,

    /// A color with which to modulate the [`GltfMaterial::reflectance`] for
    /// non-metals.
    pub specular_tint: Color,

    /// The UV channel to use for the
    /// [`GltfMaterial::specular_tint_texture`].
    #[cfg(feature = "pbr_specular_textures")]
    pub specular_tint_channel: UvChannel,

    /// A map that specifies color adjustment to be applied to the specular
    /// reflection for non-metallic materials.
    #[cfg(feature = "pbr_specular_textures")]
    pub specular_tint_texture: Option<Handle<Image>>,

    /// The amount of light transmitted _specularly_ through the material (i.e. via refraction).
    pub specular_transmission: f32,

    /// The UV channel to use for the [`GltfMaterial::specular_transmission_texture`].
    #[cfg(feature = "pbr_transmission_textures")]
    pub specular_transmission_channel: UvChannel,

    /// A map that modulates specular transmission via its red channel. Multiplied by [`GltfMaterial::specular_transmission`]
    /// to obtain the final result.
    #[cfg(feature = "pbr_transmission_textures")]
    pub specular_transmission_texture: Option<Handle<Image>>,

    /// Thickness of the volume beneath the material surface.
    pub thickness: f32,
    #[cfg(feature = "pbr_transmission_textures")]

    /// The UV channel to use for the [`GltfMaterial::thickness_texture`].
    pub thickness_channel: UvChannel,

    /// A map that modulates thickness via its green channel. Multiplied by [`GltfMaterial::thickness`]
    /// to obtain the final result.
    #[cfg(feature = "pbr_transmission_textures")]
    pub thickness_texture: Option<Handle<Image>>,

    /// The [index of refraction](https://en.wikipedia.org/wiki/Refractive_index) of the material.
    pub ior: f32,

    /// How far, on average, light travels through the volume beneath the material's
    /// surface before being absorbed.
    pub attenuation_distance: f32,

    /// The resulting (non-absorbed) color after white light travels through the attenuation distance.
    pub attenuation_color: Color,

    /// The UV channel to use for the [`GltfMaterial::normal_map_texture`].
    pub normal_map_channel: UvChannel,

    /// Used to fake the lighting of bumps and dents on a material.
    pub normal_map_texture: Option<Handle<Image>>,

    /// The UV channel to use for the [`GltfMaterial::occlusion_texture`].
    pub occlusion_channel: UvChannel,

    /// Specifies the level of exposure to ambient light.
    pub occlusion_texture: Option<Handle<Image>>,

    /// An extra thin translucent layer on top of the main PBR layer. This is
    /// typically used for painted surfaces.
    pub clearcoat: f32,

    /// The roughness of the clearcoat material. This is specified in exactly
    /// the same way as the [`GltfMaterial::perceptual_roughness`].
    pub clearcoat_perceptual_roughness: f32,

    /// The UV channel to use for the [`GltfMaterial::clearcoat_texture`].
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_channel: UvChannel,

    /// An image texture that specifies the strength of the clearcoat layer in
    /// the red channel. Values sampled from this texture are multiplied by the
    /// main [`GltfMaterial::clearcoat`] factor.
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_texture: Option<Handle<Image>>,

    /// The UV channel to use for the [`GltfMaterial::clearcoat_roughness_texture`].
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_roughness_channel: UvChannel,

    /// An image texture that specifies the roughness of the clearcoat level in
    /// the green channel. Values from this texture are multiplied by the main
    /// [`GltfMaterial::clearcoat_perceptual_roughness`] factor.
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_roughness_texture: Option<Handle<Image>>,

    /// The UV channel to use for the [`GltfMaterial::clearcoat_normal_texture`].
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_normal_channel: UvChannel,

    /// An image texture that specifies a normal map that is to be applied to
    /// the clearcoat layer. This can be used to simulate, for example,
    /// scratches on an outer layer of varnish. Normal maps are in the same
    /// format as [`GltfMaterial::normal_map_texture`].
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_normal_texture: Option<Handle<Image>>,

    /// Increases the roughness along a specific direction, so that the specular
    /// highlight will be stretched instead of being a circular lobe.
    pub anisotropy_strength: f32,

    /// The direction of increased roughness, in radians relative to the mesh
    /// tangent.
    pub anisotropy_rotation: f32,

    /// The UV channel to use for the [`GltfMaterial::anisotropy_texture`].
    #[cfg(feature = "pbr_anisotropy_texture")]
    pub anisotropy_channel: UvChannel,

    /// An image texture that allows the
    /// [`GltfMaterial::anisotropy_strength`] and
    /// [`GltfMaterial::anisotropy_rotation`] to vary across the mesh.
    #[cfg(feature = "pbr_anisotropy_texture")]
    pub anisotropy_texture: Option<Handle<Image>>,

    /// Support two-sided lighting by automatically flipping the normals for "back" faces
    /// within the PBR lighting shader.
    pub double_sided: bool,

    /// Support two-sided lighting by automatically flipping the normals for "back" faces
    /// within the PBR lighting shader.
    pub cull_mode: Option<Face>,

    /// Whether to apply only the base color to this material.
    pub unlit: bool,

    /// How to apply the alpha channel of the `base_color_texture`.
    pub alpha_mode: AlphaMode,

    /// The transform applied to the UVs corresponding to `ATTRIBUTE_UV_0` on the mesh before sampling. Default is identity.
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
