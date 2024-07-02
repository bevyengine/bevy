use bevy_asset::Asset;
use bevy_color::{Alpha, ColorToComponents};
use bevy_math::{vec2, Affine2, Affine3, Mat2, Mat3, Vec2, Vec3, Vec4};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    mesh::MeshVertexBufferLayoutRef, render_asset::RenderAssets, render_resource::*,
};
use bitflags::bitflags;

use crate::deferred::DEFAULT_PBR_DEFERRED_LIGHTING_PASS_ID;
use crate::*;

/// An enum to define which UV attribute to use for a texture.
/// It is used for every texture in the [`StandardMaterial`].
/// It only supports two UV attributes, [`Mesh::ATTRIBUTE_UV_0`] and [`Mesh::ATTRIBUTE_UV_1`].
/// The default is [`UvChannel::Uv0`].
#[derive(Reflect, Default, Debug, Clone, PartialEq, Eq)]
#[reflect(Default, Debug)]
pub enum UvChannel {
    #[default]
    Uv0,
    Uv1,
}

/// A material with "standard" properties used in PBR lighting
/// Standard property values with pictures here
/// <https://google.github.io/filament/Material%20Properties.pdf>.
///
/// May be created directly from a [`Color`] or an [`Image`].
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[bind_group_data(StandardMaterialKey)]
#[uniform(0, StandardMaterialUniform)]
#[reflect(Default, Debug)]
pub struct StandardMaterial {
    /// The color of the surface of the material before lighting.
    ///
    /// Doubles as diffuse albedo for non-metallic, specular for metallic and a mix for everything
    /// in between. If used together with a `base_color_texture`, this is factored into the final
    /// base color as `base_color * base_color_texture_value`
    ///
    /// Defaults to [`Color::WHITE`].
    pub base_color: Color,

    /// The UV channel to use for the [`StandardMaterial::base_color_texture`].
    ///
    /// Defaults to [`UvChannel::Uv0`].
    pub base_color_channel: UvChannel,

    /// The texture component of the material's color before lighting.
    /// The actual pre-lighting color is `base_color * this_texture`.
    ///
    /// See [`base_color`] for details.
    ///
    /// You should set `base_color` to [`Color::WHITE`] (the default)
    /// if you want the texture to show as-is.
    ///
    /// Setting `base_color` to something else than white will tint
    /// the texture. For example, setting `base_color` to pure red will
    /// tint the texture red.
    ///
    /// [`base_color`]: StandardMaterial::base_color
    #[texture(1)]
    #[sampler(2)]
    #[dependency]
    pub base_color_texture: Option<Handle<Image>>,

    // Use a color for user friendliness even though we technically don't use the alpha channel
    // Might be used in the future for exposure correction in HDR
    /// Color the material "emits" to the camera.
    ///
    /// This is typically used for monitor screens or LED lights.
    /// Anything that can be visible even in darkness.
    ///
    /// The emissive color is added to what would otherwise be the material's visible color.
    /// This means that for a light emissive value, in darkness,
    /// you will mostly see the emissive component.
    ///
    /// The default emissive color is [`LinearRgba::BLACK`], which doesn't add anything to the material color.
    ///
    /// To increase emissive strength, channel values for `emissive`
    /// colors can exceed `1.0`. For instance, a `base_color` of
    /// `LinearRgba::rgb(1.0, 0.0, 0.0)` represents the brightest
    /// red for objects that reflect light, but an emissive color
    /// like `LinearRgba::rgb(1000.0, 0.0, 0.0)` can be used to create
    /// intensely bright red emissive effects.
    ///
    /// Increasing the emissive strength of the color will impact visual effects
    /// like bloom, but it's important to note that **an emissive material won't
    /// light up surrounding areas like a light source**,
    /// it just adds a value to the color seen on screen.
    pub emissive: LinearRgba,

    /// The weight in which the camera exposure influences the emissive color.
    /// A value of `0.0` means the emissive color is not affected by the camera exposure.
    /// In opposition, a value of `1.0` means the emissive color is multiplied by the camera exposure.
    ///
    /// Defaults to `0.0`
    pub emissive_exposure_weight: f32,

    /// The UV channel to use for the [`StandardMaterial::emissive_texture`].
    ///
    /// Defaults to [`UvChannel::Uv0`].
    pub emissive_channel: UvChannel,

    /// The emissive map, multiplies pixels with [`emissive`]
    /// to get the final "emitting" color of a surface.
    ///
    /// This color is multiplied by [`emissive`] to get the final emitted color.
    /// Meaning that you should set [`emissive`] to [`Color::WHITE`]
    /// if you want to use the full range of color of the emissive texture.
    ///
    /// [`emissive`]: StandardMaterial::emissive
    #[texture(3)]
    #[sampler(4)]
    #[dependency]
    pub emissive_texture: Option<Handle<Image>>,

    /// Linear perceptual roughness, clamped to `[0.089, 1.0]` in the shader.
    ///
    /// Defaults to `0.5`.
    ///
    /// Low values result in a "glossy" material with specular highlights,
    /// while values close to `1` result in rough materials.
    ///
    /// If used together with a roughness/metallic texture, this is factored into the final base
    /// color as `roughness * roughness_texture_value`.
    ///
    /// 0.089 is the minimum floating point value that won't be rounded down to 0 in the
    /// calculations used.
    //
    // Technically for 32-bit floats, 0.045 could be used.
    // See <https://google.github.io/filament/Filament.html#materialsystem/parameterization/>
    pub perceptual_roughness: f32,

    /// How "metallic" the material appears, within `[0.0, 1.0]`.
    ///
    /// This should be set to 0.0 for dielectric materials or 1.0 for metallic materials.
    /// For a hybrid surface such as corroded metal, you may need to use in-between values.
    ///
    /// Defaults to `0.00`, for dielectric.
    ///
    /// If used together with a roughness/metallic texture, this is factored into the final base
    /// color as `metallic * metallic_texture_value`.
    pub metallic: f32,

    /// The UV channel to use for the [`StandardMaterial::metallic_roughness_texture`].
    ///
    /// Defaults to [`UvChannel::Uv0`].
    pub metallic_roughness_channel: UvChannel,

    /// Metallic and roughness maps, stored as a single texture.
    ///
    /// The blue channel contains metallic values,
    /// and the green channel contains the roughness values.
    /// Other channels are unused.
    ///
    /// Those values are multiplied by the scalar ones of the material,
    /// see [`metallic`] and [`perceptual_roughness`] for details.
    ///
    /// Note that with the default values of [`metallic`] and [`perceptual_roughness`],
    /// setting this texture has no effect. If you want to exclusively use the
    /// `metallic_roughness_texture` values for your material, make sure to set [`metallic`]
    /// and [`perceptual_roughness`] to `1.0`.
    ///
    /// [`metallic`]: StandardMaterial::metallic
    /// [`perceptual_roughness`]: StandardMaterial::perceptual_roughness
    #[texture(5)]
    #[sampler(6)]
    #[dependency]
    pub metallic_roughness_texture: Option<Handle<Image>>,

    /// Specular intensity for non-metals on a linear scale of `[0.0, 1.0]`.
    ///
    /// Use the value as a way to control the intensity of the
    /// specular highlight of the material, i.e. how reflective is the material,
    /// rather than the physical property "reflectance."
    ///
    /// Set to `0.0`, no specular highlight is visible, the highlight is strongest
    /// when `reflectance` is set to `1.0`.
    ///
    /// Defaults to `0.5` which is mapped to 4% reflectance in the shader.
    #[doc(alias = "specular_intensity")]
    pub reflectance: f32,

    /// The amount of light transmitted _diffusely_ through the material (i.e. “translucency”)
    ///
    /// Implemented as a second, flipped [Lambertian diffuse](https://en.wikipedia.org/wiki/Lambertian_reflectance) lobe,
    /// which provides an inexpensive but plausible approximation of translucency for thin dieletric objects (e.g. paper,
    /// leaves, some fabrics) or thicker volumetric materials with short scattering distances (e.g. porcelain, wax).
    ///
    /// For specular transmission usecases with refraction (e.g. glass) use the [`StandardMaterial::specular_transmission`] and
    /// [`StandardMaterial::ior`] properties instead.
    ///
    /// - When set to `0.0` (the default) no diffuse light is transmitted;
    /// - When set to `1.0` all diffuse light is transmitted through the material;
    /// - Values higher than `0.5` will cause more diffuse light to be transmitted than reflected, resulting in a “darker”
    ///   appearance on the side facing the light than the opposite side. (e.g. plant leaves)
    ///
    /// ## Notes
    ///
    /// - The material's [`StandardMaterial::base_color`] also modulates the transmitted light;
    /// - To receive transmitted shadows on the diffuse transmission lobe (i.e. the “backside”) of the material,
    ///   use the [`TransmittedShadowReceiver`] component.
    #[doc(alias = "translucency")]
    pub diffuse_transmission: f32,

    /// The UV channel to use for the [`StandardMaterial::diffuse_transmission_texture`].
    ///
    /// Defaults to [`UvChannel::Uv0`].
    #[cfg(feature = "pbr_transmission_textures")]
    pub diffuse_transmission_channel: UvChannel,

    /// A map that modulates diffuse transmission via its alpha channel. Multiplied by [`StandardMaterial::diffuse_transmission`]
    /// to obtain the final result.
    ///
    /// **Important:** The [`StandardMaterial::diffuse_transmission`] property must be set to a value higher than 0.0,
    /// or this texture won't have any effect.
    #[cfg_attr(feature = "pbr_transmission_textures", texture(19))]
    #[cfg_attr(feature = "pbr_transmission_textures", sampler(20))]
    #[cfg(feature = "pbr_transmission_textures")]
    pub diffuse_transmission_texture: Option<Handle<Image>>,

    /// The amount of light transmitted _specularly_ through the material (i.e. via refraction)
    ///
    /// - When set to `0.0` (the default) no light is transmitted.
    /// - When set to `1.0` all light is transmitted through the material.
    ///
    /// The material's [`StandardMaterial::base_color`] also modulates the transmitted light.
    ///
    /// **Note:** Typically used in conjunction with [`StandardMaterial::thickness`], [`StandardMaterial::ior`] and [`StandardMaterial::perceptual_roughness`].
    ///
    /// ## Performance
    ///
    /// Specular transmission is implemented as a relatively expensive screen-space effect that allows ocluded objects to be seen through the material,
    /// with distortion and blur effects.
    ///
    /// - [`Camera3d::screen_space_specular_transmission_steps`](bevy_core_pipeline::core_3d::Camera3d::screen_space_specular_transmission_steps) can be used to enable transmissive objects
    /// to be seen through other transmissive objects, at the cost of additional draw calls and texture copies; (Use with caution!)
    ///     - If a simplified approximation of specular transmission using only environment map lighting is sufficient, consider setting
    /// [`Camera3d::screen_space_specular_transmission_steps`](bevy_core_pipeline::core_3d::Camera3d::screen_space_specular_transmission_steps) to `0`.
    /// - If purely diffuse light transmission is needed, (i.e. “translucency”) consider using [`StandardMaterial::diffuse_transmission`] instead,
    /// for a much less expensive effect.
    /// - Specular transmission is rendered before alpha blending, so any material with [`AlphaMode::Blend`], [`AlphaMode::Premultiplied`], [`AlphaMode::Add`] or [`AlphaMode::Multiply`]
    ///   won't be visible through specular transmissive materials.
    #[doc(alias = "refraction")]
    pub specular_transmission: f32,

    /// The UV channel to use for the [`StandardMaterial::specular_transmission_texture`].
    ///
    /// Defaults to [`UvChannel::Uv0`].
    #[cfg(feature = "pbr_transmission_textures")]
    pub specular_transmission_channel: UvChannel,

    /// A map that modulates specular transmission via its red channel. Multiplied by [`StandardMaterial::specular_transmission`]
    /// to obtain the final result.
    ///
    /// **Important:** The [`StandardMaterial::specular_transmission`] property must be set to a value higher than 0.0,
    /// or this texture won't have any effect.
    #[cfg_attr(feature = "pbr_transmission_textures", texture(15))]
    #[cfg_attr(feature = "pbr_transmission_textures", sampler(16))]
    #[cfg(feature = "pbr_transmission_textures")]
    pub specular_transmission_texture: Option<Handle<Image>>,

    /// Thickness of the volume beneath the material surface.
    ///
    /// When set to `0.0` (the default) the material appears as an infinitely-thin film,
    /// transmitting light without distorting it.
    ///
    /// When set to any other value, the material distorts light like a thick lens.
    ///
    /// **Note:** Typically used in conjunction with [`StandardMaterial::specular_transmission`] and [`StandardMaterial::ior`], or with
    /// [`StandardMaterial::diffuse_transmission`].
    #[doc(alias = "volume")]
    #[doc(alias = "thin_walled")]
    pub thickness: f32,

    /// The UV channel to use for the [`StandardMaterial::thickness_texture`].
    ///
    /// Defaults to [`UvChannel::Uv0`].
    #[cfg(feature = "pbr_transmission_textures")]
    pub thickness_channel: UvChannel,

    /// A map that modulates thickness via its green channel. Multiplied by [`StandardMaterial::thickness`]
    /// to obtain the final result.
    ///
    /// **Important:** The [`StandardMaterial::thickness`] property must be set to a value higher than 0.0,
    /// or this texture won't have any effect.
    #[cfg_attr(feature = "pbr_transmission_textures", texture(17))]
    #[cfg_attr(feature = "pbr_transmission_textures", sampler(18))]
    #[cfg(feature = "pbr_transmission_textures")]
    pub thickness_texture: Option<Handle<Image>>,

    /// The [index of refraction](https://en.wikipedia.org/wiki/Refractive_index) of the material.
    ///
    /// Defaults to 1.5.
    ///
    /// | Material        | Index of Refraction  |
    /// |:----------------|:---------------------|
    /// | Vacuum          | 1                    |
    /// | Air             | 1.00                 |
    /// | Ice             | 1.31                 |
    /// | Water           | 1.33                 |
    /// | Eyes            | 1.38                 |
    /// | Quartz          | 1.46                 |
    /// | Olive Oil       | 1.47                 |
    /// | Honey           | 1.49                 |
    /// | Acrylic         | 1.49                 |
    /// | Window Glass    | 1.52                 |
    /// | Polycarbonate   | 1.58                 |
    /// | Flint Glass     | 1.69                 |
    /// | Ruby            | 1.71                 |
    /// | Glycerine       | 1.74                 |
    /// | Sapphire        | 1.77                 |
    /// | Cubic Zirconia  | 2.15                 |
    /// | Diamond         | 2.42                 |
    /// | Moissanite      | 2.65                 |
    ///
    /// **Note:** Typically used in conjunction with [`StandardMaterial::specular_transmission`] and [`StandardMaterial::thickness`].
    #[doc(alias = "index_of_refraction")]
    #[doc(alias = "refraction_index")]
    #[doc(alias = "refractive_index")]
    pub ior: f32,

    /// How far, on average, light travels through the volume beneath the material's
    /// surface before being absorbed.
    ///
    /// Defaults to [`f32::INFINITY`], i.e. light is never absorbed.
    ///
    /// **Note:** To have any effect, must be used in conjunction with:
    /// - [`StandardMaterial::attenuation_color`];
    /// - [`StandardMaterial::thickness`];
    /// - [`StandardMaterial::diffuse_transmission`] or [`StandardMaterial::specular_transmission`].
    #[doc(alias = "absorption_distance")]
    #[doc(alias = "extinction_distance")]
    pub attenuation_distance: f32,

    /// The resulting (non-absorbed) color after white light travels through the attenuation distance.
    ///
    /// Defaults to [`Color::WHITE`], i.e. no change.
    ///
    /// **Note:** To have any effect, must be used in conjunction with:
    /// - [`StandardMaterial::attenuation_distance`];
    /// - [`StandardMaterial::thickness`];
    /// - [`StandardMaterial::diffuse_transmission`] or [`StandardMaterial::specular_transmission`].
    #[doc(alias = "absorption_color")]
    #[doc(alias = "extinction_color")]
    pub attenuation_color: Color,

    /// The UV channel to use for the [`StandardMaterial::normal_map_texture`].
    ///
    /// Defaults to [`UvChannel::Uv0`].
    pub normal_map_channel: UvChannel,

    /// Used to fake the lighting of bumps and dents on a material.
    ///
    /// A typical usage would be faking cobblestones on a flat plane mesh in 3D.
    ///
    /// # Notes
    ///
    /// Normal mapping with `StandardMaterial` and the core bevy PBR shaders requires:
    /// - A normal map texture
    /// - Vertex UVs
    /// - Vertex tangents
    /// - Vertex normals
    ///
    /// Tangents do not have to be stored in your model,
    /// they can be generated using the [`Mesh::generate_tangents`] or
    /// [`Mesh::with_generated_tangents`] methods.
    /// If your material has a normal map, but still renders as a flat surface,
    /// make sure your meshes have their tangents set.
    ///
    /// [`Mesh::generate_tangents`]: bevy_render::mesh::Mesh::generate_tangents
    /// [`Mesh::with_generated_tangents`]: bevy_render::mesh::Mesh::with_generated_tangents
    #[texture(9)]
    #[sampler(10)]
    #[dependency]
    pub normal_map_texture: Option<Handle<Image>>,

    /// Normal map textures authored for DirectX have their y-component flipped. Set this to flip
    /// it to right-handed conventions.
    pub flip_normal_map_y: bool,

    /// The UV channel to use for the [`StandardMaterial::occlusion_texture`].
    ///
    /// Defaults to [`UvChannel::Uv0`].
    pub occlusion_channel: UvChannel,

    /// Specifies the level of exposure to ambient light.
    ///
    /// This is usually generated and stored automatically ("baked") by 3D-modelling software.
    ///
    /// Typically, steep concave parts of a model (such as the armpit of a shirt) are darker,
    /// because they have little exposure to light.
    /// An occlusion map specifies those parts of the model that light doesn't reach well.
    ///
    /// The material will be less lit in places where this texture is dark.
    /// This is similar to ambient occlusion, but built into the model.
    #[texture(7)]
    #[sampler(8)]
    #[dependency]
    pub occlusion_texture: Option<Handle<Image>>,

    /// An extra thin translucent layer on top of the main PBR layer. This is
    /// typically used for painted surfaces.
    ///
    /// This value specifies the strength of the layer, which affects how
    /// visible the clearcoat layer will be.
    ///
    /// Defaults to zero, specifying no clearcoat layer.
    pub clearcoat: f32,

    /// The UV channel to use for the [`StandardMaterial::clearcoat_texture`].
    ///
    /// Defaults to [`UvChannel::Uv0`].
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_channel: UvChannel,

    /// An image texture that specifies the strength of the clearcoat layer in
    /// the red channel. Values sampled from this texture are multiplied by the
    /// main [`StandardMaterial::clearcoat`] factor.
    ///
    /// As this is a non-color map, it must not be loaded as sRGB.
    #[cfg_attr(feature = "pbr_multi_layer_material_textures", texture(21))]
    #[cfg_attr(feature = "pbr_multi_layer_material_textures", sampler(22))]
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_texture: Option<Handle<Image>>,

    /// The roughness of the clearcoat material. This is specified in exactly
    /// the same way as the [`StandardMaterial::perceptual_roughness`].
    ///
    /// If the [`StandardMaterial::clearcoat`] value if zero, this has no
    /// effect.
    ///
    /// Defaults to 0.5.
    pub clearcoat_perceptual_roughness: f32,

    /// The UV channel to use for the [`StandardMaterial::clearcoat_roughness_texture`].
    ///
    /// Defaults to [`UvChannel::Uv0`].
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_roughness_channel: UvChannel,

    /// An image texture that specifies the roughness of the clearcoat level in
    /// the green channel. Values from this texture are multiplied by the main
    /// [`StandardMaterial::clearcoat_perceptual_roughness`] factor.
    ///
    /// As this is a non-color map, it must not be loaded as sRGB.
    #[cfg_attr(feature = "pbr_multi_layer_material_textures", texture(23))]
    #[cfg_attr(feature = "pbr_multi_layer_material_textures", sampler(24))]
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_roughness_texture: Option<Handle<Image>>,

    /// The UV channel to use for the [`StandardMaterial::clearcoat_normal_texture`].
    ///
    /// Defaults to [`UvChannel::Uv0`].
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_normal_channel: UvChannel,

    /// An image texture that specifies a normal map that is to be applied to
    /// the clearcoat layer. This can be used to simulate, for example,
    /// scratches on an outer layer of varnish. Normal maps are in the same
    /// format as [`StandardMaterial::normal_map_texture`].
    ///
    /// Note that, if a clearcoat normal map isn't specified, the main normal
    /// map, if any, won't be applied to the clearcoat. If you want a normal map
    /// that applies to both the main materal and to the clearcoat, specify it
    /// in both [`StandardMaterial::normal_map_texture`] and this field.
    ///
    /// As this is a non-color map, it must not be loaded as sRGB.
    #[cfg_attr(feature = "pbr_multi_layer_material_textures", texture(25))]
    #[cfg_attr(feature = "pbr_multi_layer_material_textures", sampler(26))]
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    pub clearcoat_normal_texture: Option<Handle<Image>>,

    /// Increases the roughness along a specific direction, so that the specular
    /// highlight will be stretched instead of being a circular lobe.
    ///
    /// This value ranges from 0 (perfectly circular) to 1 (maximally
    /// stretched). The default direction (corresponding to a
    /// [`StandardMaterial::anisotropy_rotation`] of 0) aligns with the
    /// *tangent* of the mesh; thus mesh tangents must be specified in order for
    /// this parameter to have any meaning. The direction can be changed using
    /// the [`StandardMaterial::anisotropy_rotation`] parameter.
    ///
    /// This is typically used for modeling surfaces such as brushed metal and
    /// hair, in which one direction of the surface but not the other is smooth.
    ///
    /// See the [`KHR_materials_anisotropy` specification] for more details.
    ///
    /// [`KHR_materials_anisotropy` specification]:
    /// https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_anisotropy/README.md
    pub anisotropy_strength: f32,

    /// The direction of increased roughness, in radians relative to the mesh
    /// tangent.
    ///
    /// This parameter causes the roughness to vary according to the
    /// [`StandardMaterial::anisotropy_strength`]. The rotation is applied in
    /// tangent-bitangent space; thus, mesh tangents must be present for this
    /// parameter to have any meaning.
    ///
    /// This parameter has no effect if
    /// [`StandardMaterial::anisotropy_strength`] is zero. Its value can
    /// optionally be adjusted across the mesh with the
    /// [`StandardMaterial::anisotropy_texture`].
    ///
    /// See the [`KHR_materials_anisotropy` specification] for more details.
    ///
    /// [`KHR_materials_anisotropy` specification]:
    /// https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_anisotropy/README.md
    pub anisotropy_rotation: f32,

    /// The UV channel to use for the [`StandardMaterial::anisotropy_texture`].
    ///
    /// Defaults to [`UvChannel::Uv0`].
    #[cfg(feature = "pbr_anisotropy_texture")]
    pub anisotropy_channel: UvChannel,

    /// An image texture that allows the
    /// [`StandardMaterial::anisotropy_strength`] and
    /// [`StandardMaterial::anisotropy_rotation`] to vary across the mesh.
    ///
    /// The [`KHR_materials_anisotropy` specification] defines the format that
    /// this texture must take. To summarize: The direction vector is encoded in
    /// the red and green channels, while the strength is encoded in the blue
    /// channels. For the direction vector, the red and green channels map the
    /// color range [0, 1] to the vector range [-1, 1]. The direction vector
    /// encoded in this texture modifies the default rotation direction in
    /// tangent-bitangent space, before the
    /// [`StandardMaterial::anisotropy_rotation`] parameter is applied. The
    /// value in the blue channel is multiplied by the
    /// [`StandardMaterial::anisotropy_strength`] value to produce the final
    /// anisotropy strength.
    ///
    /// As the texel values don't represent colors, this texture must be in
    /// linear color space, not sRGB.
    ///
    /// [`KHR_materials_anisotropy` specification]:
    /// https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_anisotropy/README.md
    #[cfg_attr(feature = "pbr_anisotropy_texture", texture(13))]
    #[cfg_attr(feature = "pbr_anisotropy_texture", sampler(14))]
    #[cfg(feature = "pbr_anisotropy_texture")]
    pub anisotropy_texture: Option<Handle<Image>>,

    /// Support two-sided lighting by automatically flipping the normals for "back" faces
    /// within the PBR lighting shader.
    ///
    /// Defaults to `false`.
    /// This does not automatically configure backface culling,
    /// which can be done via `cull_mode`.
    pub double_sided: bool,

    /// Whether to cull the "front", "back" or neither side of a mesh.
    /// If set to `None`, the two sides of the mesh are visible.
    ///
    /// Defaults to `Some(Face::Back)`.
    /// In bevy, the order of declaration of a triangle's vertices
    /// in [`Mesh`] defines the triangle's front face.
    ///
    /// When a triangle is in a viewport,
    /// if its vertices appear counter-clockwise from the viewport's perspective,
    /// then the viewport is seeing the triangle's front face.
    /// Conversely, if the vertices appear clockwise, you are seeing the back face.
    ///
    /// In short, in bevy, front faces winds counter-clockwise.
    ///
    /// Your 3D editing software should manage all of that.
    ///
    /// [`Mesh`]: bevy_render::mesh::Mesh
    // TODO: include this in reflection somehow (maybe via remote types like serde https://serde.rs/remote-derive.html)
    #[reflect(ignore)]
    pub cull_mode: Option<Face>,

    /// Whether to apply only the base color to this material.
    ///
    /// Normals, occlusion textures, roughness, metallic, reflectance, emissive,
    /// shadows, alpha mode and ambient light are ignored if this is set to `true`.
    pub unlit: bool,

    /// Whether to enable fog for this material.
    pub fog_enabled: bool,

    /// How to apply the alpha channel of the `base_color_texture`.
    ///
    /// See [`AlphaMode`] for details. Defaults to [`AlphaMode::Opaque`].
    pub alpha_mode: AlphaMode,

    /// Adjust rendered depth.
    ///
    /// A material with a positive depth bias will render closer to the
    /// camera while negative values cause the material to render behind
    /// other objects. This is independent of the viewport.
    ///
    /// `depth_bias` affects render ordering and depth write operations
    /// using the `wgpu::DepthBiasState::Constant` field.
    ///
    /// [z-fighting]: https://en.wikipedia.org/wiki/Z-fighting
    pub depth_bias: f32,

    /// The depth map used for [parallax mapping].
    ///
    /// It is a greyscale image where white represents bottom and black the top.
    /// If this field is set, bevy will apply [parallax mapping].
    /// Parallax mapping, unlike simple normal maps, will move the texture
    /// coordinate according to the current perspective,
    /// giving actual depth to the texture.
    ///
    /// The visual result is similar to a displacement map,
    /// but does not require additional geometry.
    ///
    /// Use the [`parallax_depth_scale`] field to control the depth of the parallax.
    ///
    /// ## Limitations
    ///
    /// - It will look weird on bent/non-planar surfaces.
    /// - The depth of the pixel does not reflect its visual position, resulting
    ///   in artifacts for depth-dependent features such as fog or SSAO.
    /// - For the same reason, the geometry silhouette will always be
    ///   the one of the actual geometry, not the parallaxed version, resulting
    ///   in awkward looks on intersecting parallaxed surfaces.
    ///
    /// ## Performance
    ///
    /// Parallax mapping requires multiple texture lookups, proportional to
    /// [`max_parallax_layer_count`], which might be costly.
    ///
    /// Use the [`parallax_mapping_method`] and [`max_parallax_layer_count`] fields
    /// to tweak the shader, trading graphical quality for performance.
    ///
    /// To improve performance, set your `depth_map`'s [`Image::sampler`]
    /// filter mode to `FilterMode::Nearest`, as [this paper] indicates, it improves
    /// performance a bit.
    ///
    /// To reduce artifacts, avoid steep changes in depth, blurring the depth
    /// map helps with this.
    ///
    /// Larger depth maps haves a disproportionate performance impact.
    ///
    /// [this paper]: https://www.diva-portal.org/smash/get/diva2:831762/FULLTEXT01.pdf
    /// [parallax mapping]: https://en.wikipedia.org/wiki/Parallax_mapping
    /// [`parallax_depth_scale`]: StandardMaterial::parallax_depth_scale
    /// [`parallax_mapping_method`]: StandardMaterial::parallax_mapping_method
    /// [`max_parallax_layer_count`]: StandardMaterial::max_parallax_layer_count
    #[texture(11)]
    #[sampler(12)]
    #[dependency]
    pub depth_map: Option<Handle<Image>>,

    /// How deep the offset introduced by the depth map should be.
    ///
    /// Default is `0.1`, anything over that value may look distorted.
    /// Lower values lessen the effect.
    ///
    /// The depth is relative to texture size. This means that if your texture
    /// occupies a surface of `1` world unit, and `parallax_depth_scale` is `0.1`, then
    /// the in-world depth will be of `0.1` world units.
    /// If the texture stretches for `10` world units, then the final depth
    /// will be of `1` world unit.
    pub parallax_depth_scale: f32,

    /// Which parallax mapping method to use.
    ///
    /// We recommend that all objects use the same [`ParallaxMappingMethod`], to avoid
    /// duplicating and running two shaders.
    pub parallax_mapping_method: ParallaxMappingMethod,

    /// In how many layers to split the depth maps for parallax mapping.
    ///
    /// If you are seeing jaggy edges, increase this value.
    /// However, this incurs a performance cost.
    ///
    /// Dependent on the situation, switching to [`ParallaxMappingMethod::Relief`]
    /// and keeping this value low might have better performance than increasing the
    /// layer count while using [`ParallaxMappingMethod::Occlusion`].
    ///
    /// Default is `16.0`.
    pub max_parallax_layer_count: f32,

    /// The exposure (brightness) level of the lightmap, if present.
    pub lightmap_exposure: f32,

    /// Render method used for opaque materials. (Where `alpha_mode` is [`AlphaMode::Opaque`] or [`AlphaMode::Mask`])
    pub opaque_render_method: OpaqueRendererMethod,

    /// Used for selecting the deferred lighting pass for deferred materials.
    /// Default is [`DEFAULT_PBR_DEFERRED_LIGHTING_PASS_ID`] for default
    /// PBR deferred lighting pass. Ignored in the case of forward materials.
    pub deferred_lighting_pass_id: u8,

    /// The transform applied to the UVs corresponding to `ATTRIBUTE_UV_0` on the mesh before sampling. Default is identity.
    pub uv_transform: Affine2,
}

impl StandardMaterial {
    /// Horizontal flipping transform
    ///
    /// Multiplying this with another Affine2 returns transformation with horizontally flipped texture coords
    pub const FLIP_HORIZONTAL: Affine2 = Affine2 {
        matrix2: Mat2::from_cols(Vec2::new(-1.0, 0.0), Vec2::Y),
        translation: Vec2::X,
    };

    /// Vertical flipping transform
    ///
    /// Multiplying this with another Affine2 returns transformation with vertically flipped texture coords
    pub const FLIP_VERTICAL: Affine2 = Affine2 {
        matrix2: Mat2::from_cols(Vec2::X, Vec2::new(0.0, -1.0)),
        translation: Vec2::Y,
    };

    /// Flipping X 3D transform
    ///
    /// Multiplying this with another Affine3 returns transformation with flipped X coords
    pub const FLIP_X: Affine3 = Affine3 {
        matrix3: Mat3::from_cols(Vec3::new(-1.0, 0.0, 0.0), Vec3::Y, Vec3::Z),
        translation: Vec3::X,
    };

    /// Flipping Y 3D transform
    ///
    /// Multiplying this with another Affine3 returns transformation with flipped Y coords
    pub const FLIP_Y: Affine3 = Affine3 {
        matrix3: Mat3::from_cols(Vec3::X, Vec3::new(0.0, -1.0, 0.0), Vec3::Z),
        translation: Vec3::Y,
    };

    /// Flipping Z 3D transform
    ///
    /// Multiplying this with another Affine3 returns transformation with flipped Z coords
    pub const FLIP_Z: Affine3 = Affine3 {
        matrix3: Mat3::from_cols(Vec3::X, Vec3::Y, Vec3::new(0.0, 0.0, -1.0)),
        translation: Vec3::Z,
    };

    /// Flip the texture coordinates of the material.
    pub fn flip(&mut self, horizontal: bool, vertical: bool) {
        if horizontal {
            // Multiplication of `Affine2` is order dependent, which is why
            // we do not use the `*=` operator.
            self.uv_transform = Self::FLIP_HORIZONTAL * self.uv_transform;
        }
        if vertical {
            self.uv_transform = Self::FLIP_VERTICAL * self.uv_transform;
        }
    }

    /// Consumes the material and returns a material with flipped texture coordinates
    pub fn flipped(mut self, horizontal: bool, vertical: bool) -> Self {
        self.flip(horizontal, vertical);
        self
    }

    /// Creates a new material from a given color
    pub fn from_color(color: impl Into<Color>) -> Self {
        Self::from(color.into())
    }
}

impl Default for StandardMaterial {
    fn default() -> Self {
        StandardMaterial {
            // White because it gets multiplied with texture values if someone uses
            // a texture.
            base_color: Color::WHITE,
            base_color_channel: UvChannel::Uv0,
            base_color_texture: None,
            emissive: LinearRgba::BLACK,
            emissive_exposure_weight: 0.0,
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
            diffuse_transmission: 0.0,
            #[cfg(feature = "pbr_transmission_textures")]
            diffuse_transmission_channel: UvChannel::Uv0,
            #[cfg(feature = "pbr_transmission_textures")]
            diffuse_transmission_texture: None,
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
            flip_normal_map_y: false,
            double_sided: false,
            cull_mode: Some(Face::Back),
            unlit: false,
            fog_enabled: true,
            alpha_mode: AlphaMode::Opaque,
            depth_bias: 0.0,
            depth_map: None,
            parallax_depth_scale: 0.1,
            max_parallax_layer_count: 16.0,
            lightmap_exposure: 1.0,
            parallax_mapping_method: ParallaxMappingMethod::Occlusion,
            opaque_render_method: OpaqueRendererMethod::Auto,
            deferred_lighting_pass_id: DEFAULT_PBR_DEFERRED_LIGHTING_PASS_ID,
            uv_transform: Affine2::IDENTITY,
        }
    }
}

impl From<Color> for StandardMaterial {
    fn from(color: Color) -> Self {
        StandardMaterial {
            base_color: color,
            alpha_mode: if color.alpha() < 1.0 {
                AlphaMode::Blend
            } else {
                AlphaMode::Opaque
            },
            ..Default::default()
        }
    }
}

impl From<Handle<Image>> for StandardMaterial {
    fn from(texture: Handle<Image>) -> Self {
        StandardMaterial {
            base_color_texture: Some(texture),
            ..Default::default()
        }
    }
}

// NOTE: These must match the bit flags in bevy_pbr/src/render/pbr_types.wgsl!
bitflags::bitflags! {
    /// Bitflags info about the material a shader is currently rendering.
    /// This is accessible in the shader in the [`StandardMaterialUniform`]
    #[repr(transparent)]
    pub struct StandardMaterialFlags: u32 {
        const BASE_COLOR_TEXTURE         = 1 << 0;
        const EMISSIVE_TEXTURE           = 1 << 1;
        const METALLIC_ROUGHNESS_TEXTURE = 1 << 2;
        const OCCLUSION_TEXTURE          = 1 << 3;
        const DOUBLE_SIDED               = 1 << 4;
        const UNLIT                      = 1 << 5;
        const TWO_COMPONENT_NORMAL_MAP   = 1 << 6;
        const FLIP_NORMAL_MAP_Y          = 1 << 7;
        const FOG_ENABLED                = 1 << 8;
        const DEPTH_MAP                  = 1 << 9; // Used for parallax mapping
        const SPECULAR_TRANSMISSION_TEXTURE = 1 << 10;
        const THICKNESS_TEXTURE          = 1 << 11;
        const DIFFUSE_TRANSMISSION_TEXTURE = 1 << 12;
        const ATTENUATION_ENABLED        = 1 << 13;
        const CLEARCOAT_TEXTURE          = 1 << 14;
        const CLEARCOAT_ROUGHNESS_TEXTURE = 1 << 15;
        const CLEARCOAT_NORMAL_TEXTURE   = 1 << 16;
        const ANISOTROPY_TEXTURE         = 1 << 17;
        const ALPHA_MODE_RESERVED_BITS   = Self::ALPHA_MODE_MASK_BITS << Self::ALPHA_MODE_SHIFT_BITS; // ← Bitmask reserving bits for the `AlphaMode`
        const ALPHA_MODE_OPAQUE          = 0 << Self::ALPHA_MODE_SHIFT_BITS;                          // ← Values are just sequential values bitshifted into
        const ALPHA_MODE_MASK            = 1 << Self::ALPHA_MODE_SHIFT_BITS;                          //   the bitmask, and can range from 0 to 7.
        const ALPHA_MODE_BLEND           = 2 << Self::ALPHA_MODE_SHIFT_BITS;                          //
        const ALPHA_MODE_PREMULTIPLIED   = 3 << Self::ALPHA_MODE_SHIFT_BITS;                          //
        const ALPHA_MODE_ADD             = 4 << Self::ALPHA_MODE_SHIFT_BITS;                          //   Right now only values 0–5 are used, which still gives
        const ALPHA_MODE_MULTIPLY        = 5 << Self::ALPHA_MODE_SHIFT_BITS;                          // ← us "room" for two more modes without adding more bits
        const ALPHA_MODE_ALPHA_TO_COVERAGE = 6 << Self::ALPHA_MODE_SHIFT_BITS;
        const NONE                       = 0;
        const UNINITIALIZED              = 0xFFFF;
    }
}

impl StandardMaterialFlags {
    const ALPHA_MODE_MASK_BITS: u32 = 0b111;
    const ALPHA_MODE_SHIFT_BITS: u32 = 32 - Self::ALPHA_MODE_MASK_BITS.count_ones();
}

/// The GPU representation of the uniform data of a [`StandardMaterial`].
#[derive(Clone, Default, ShaderType)]
pub struct StandardMaterialUniform {
    /// Doubles as diffuse albedo for non-metallic, specular for metallic and a mix for everything
    /// in between.
    pub base_color: Vec4,
    // Use a color for user-friendliness even though we technically don't use the alpha channel
    // Might be used in the future for exposure correction in HDR
    pub emissive: Vec4,
    /// Color white light takes after travelling through the attenuation distance underneath the material surface
    pub attenuation_color: Vec4,
    /// The transform applied to the UVs corresponding to `ATTRIBUTE_UV_0` on the mesh before sampling. Default is identity.
    pub uv_transform: Mat3,
    /// Linear perceptual roughness, clamped to [0.089, 1.0] in the shader
    /// Defaults to minimum of 0.089
    pub roughness: f32,
    /// From [0.0, 1.0], dielectric to pure metallic
    pub metallic: f32,
    /// Specular intensity for non-metals on a linear scale of [0.0, 1.0]
    /// defaults to 0.5 which is mapped to 4% reflectance in the shader
    pub reflectance: f32,
    /// Amount of diffuse light transmitted through the material
    pub diffuse_transmission: f32,
    /// Amount of specular light transmitted through the material
    pub specular_transmission: f32,
    /// Thickness of the volume underneath the material surface
    pub thickness: f32,
    /// Index of Refraction
    pub ior: f32,
    /// How far light travels through the volume underneath the material surface before being absorbed
    pub attenuation_distance: f32,
    pub clearcoat: f32,
    pub clearcoat_perceptual_roughness: f32,
    pub anisotropy_strength: f32,
    pub anisotropy_rotation: Vec2,
    /// The [`StandardMaterialFlags`] accessible in the `wgsl` shader.
    pub flags: u32,
    /// When the alpha mode mask flag is set, any base color alpha above this cutoff means fully opaque,
    /// and any below means fully transparent.
    pub alpha_cutoff: f32,
    /// The depth of the [`StandardMaterial::depth_map`] to apply.
    pub parallax_depth_scale: f32,
    /// In how many layers to split the depth maps for Steep parallax mapping.
    ///
    /// If your `parallax_depth_scale` is >0.1 and you are seeing jaggy edges,
    /// increase this value. However, this incurs a performance cost.
    pub max_parallax_layer_count: f32,
    /// The exposure (brightness) level of the lightmap, if present.
    pub lightmap_exposure: f32,
    /// Using [`ParallaxMappingMethod::Relief`], how many additional
    /// steps to use at most to find the depth value.
    pub max_relief_mapping_search_steps: u32,
    /// ID for specifying which deferred lighting pass should be used for rendering this material, if any.
    pub deferred_lighting_pass_id: u32,
}

impl AsBindGroupShaderType<StandardMaterialUniform> for StandardMaterial {
    fn as_bind_group_shader_type(
        &self,
        images: &RenderAssets<GpuImage>,
    ) -> StandardMaterialUniform {
        let mut flags = StandardMaterialFlags::NONE;
        if self.base_color_texture.is_some() {
            flags |= StandardMaterialFlags::BASE_COLOR_TEXTURE;
        }
        if self.emissive_texture.is_some() {
            flags |= StandardMaterialFlags::EMISSIVE_TEXTURE;
        }
        if self.metallic_roughness_texture.is_some() {
            flags |= StandardMaterialFlags::METALLIC_ROUGHNESS_TEXTURE;
        }
        if self.occlusion_texture.is_some() {
            flags |= StandardMaterialFlags::OCCLUSION_TEXTURE;
        }
        if self.double_sided {
            flags |= StandardMaterialFlags::DOUBLE_SIDED;
        }
        if self.unlit {
            flags |= StandardMaterialFlags::UNLIT;
        }
        if self.fog_enabled {
            flags |= StandardMaterialFlags::FOG_ENABLED;
        }
        if self.depth_map.is_some() {
            flags |= StandardMaterialFlags::DEPTH_MAP;
        }
        #[cfg(feature = "pbr_transmission_textures")]
        {
            if self.specular_transmission_texture.is_some() {
                flags |= StandardMaterialFlags::SPECULAR_TRANSMISSION_TEXTURE;
            }
            if self.thickness_texture.is_some() {
                flags |= StandardMaterialFlags::THICKNESS_TEXTURE;
            }
            if self.diffuse_transmission_texture.is_some() {
                flags |= StandardMaterialFlags::DIFFUSE_TRANSMISSION_TEXTURE;
            }
        }

        #[cfg(feature = "pbr_anisotropy_texture")]
        {
            if self.anisotropy_texture.is_some() {
                flags |= StandardMaterialFlags::ANISOTROPY_TEXTURE;
            }
        }

        #[cfg(feature = "pbr_multi_layer_material_textures")]
        {
            if self.clearcoat_texture.is_some() {
                flags |= StandardMaterialFlags::CLEARCOAT_TEXTURE;
            }
            if self.clearcoat_roughness_texture.is_some() {
                flags |= StandardMaterialFlags::CLEARCOAT_ROUGHNESS_TEXTURE;
            }
            if self.clearcoat_normal_texture.is_some() {
                flags |= StandardMaterialFlags::CLEARCOAT_NORMAL_TEXTURE;
            }
        }

        let has_normal_map = self.normal_map_texture.is_some();
        if has_normal_map {
            let normal_map_id = self.normal_map_texture.as_ref().map(|h| h.id()).unwrap();
            if let Some(texture) = images.get(normal_map_id) {
                match texture.texture_format {
                    // All 2-component unorm formats
                    TextureFormat::Rg8Unorm
                    | TextureFormat::Rg16Unorm
                    | TextureFormat::Bc5RgUnorm
                    | TextureFormat::EacRg11Unorm => {
                        flags |= StandardMaterialFlags::TWO_COMPONENT_NORMAL_MAP;
                    }
                    _ => {}
                }
            }
            if self.flip_normal_map_y {
                flags |= StandardMaterialFlags::FLIP_NORMAL_MAP_Y;
            }
        }
        // NOTE: 0.5 is from the glTF default - do we want this?
        let mut alpha_cutoff = 0.5;
        match self.alpha_mode {
            AlphaMode::Opaque => flags |= StandardMaterialFlags::ALPHA_MODE_OPAQUE,
            AlphaMode::Mask(c) => {
                alpha_cutoff = c;
                flags |= StandardMaterialFlags::ALPHA_MODE_MASK;
            }
            AlphaMode::Blend => flags |= StandardMaterialFlags::ALPHA_MODE_BLEND,
            AlphaMode::Premultiplied => flags |= StandardMaterialFlags::ALPHA_MODE_PREMULTIPLIED,
            AlphaMode::Add => flags |= StandardMaterialFlags::ALPHA_MODE_ADD,
            AlphaMode::Multiply => flags |= StandardMaterialFlags::ALPHA_MODE_MULTIPLY,
            AlphaMode::AlphaToCoverage => {
                flags |= StandardMaterialFlags::ALPHA_MODE_ALPHA_TO_COVERAGE;
            }
        };

        if self.attenuation_distance.is_finite() {
            flags |= StandardMaterialFlags::ATTENUATION_ENABLED;
        }

        let mut emissive = self.emissive.to_vec4();
        emissive[3] = self.emissive_exposure_weight;

        // Doing this up front saves having to do this repeatedly in the fragment shader.
        let anisotropy_rotation = vec2(
            self.anisotropy_rotation.cos(),
            self.anisotropy_rotation.sin(),
        );

        StandardMaterialUniform {
            base_color: LinearRgba::from(self.base_color).to_vec4(),
            emissive,
            roughness: self.perceptual_roughness,
            metallic: self.metallic,
            reflectance: self.reflectance,
            clearcoat: self.clearcoat,
            clearcoat_perceptual_roughness: self.clearcoat_perceptual_roughness,
            anisotropy_strength: self.anisotropy_strength,
            anisotropy_rotation,
            diffuse_transmission: self.diffuse_transmission,
            specular_transmission: self.specular_transmission,
            thickness: self.thickness,
            ior: self.ior,
            attenuation_distance: self.attenuation_distance,
            attenuation_color: LinearRgba::from(self.attenuation_color)
                .to_f32_array()
                .into(),
            flags: flags.bits(),
            alpha_cutoff,
            parallax_depth_scale: self.parallax_depth_scale,
            max_parallax_layer_count: self.max_parallax_layer_count,
            lightmap_exposure: self.lightmap_exposure,
            max_relief_mapping_search_steps: self.parallax_mapping_method.max_steps(),
            deferred_lighting_pass_id: self.deferred_lighting_pass_id as u32,
            uv_transform: self.uv_transform.into(),
        }
    }
}

bitflags! {
    /// The pipeline key for `StandardMaterial`, packed into 64 bits.
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    pub struct StandardMaterialKey: u64 {
        const CULL_FRONT               = 0x000001;
        const CULL_BACK                = 0x000002;
        const NORMAL_MAP               = 0x000004;
        const RELIEF_MAPPING           = 0x000008;
        const DIFFUSE_TRANSMISSION     = 0x000010;
        const SPECULAR_TRANSMISSION    = 0x000020;
        const CLEARCOAT                = 0x000040;
        const CLEARCOAT_NORMAL_MAP     = 0x000080;
        const ANISOTROPY               = 0x000100;
        const BASE_COLOR_UV            = 0x000200;
        const EMISSIVE_UV              = 0x000400;
        const METALLIC_ROUGHNESS_UV    = 0x000800;
        const OCCLUSION_UV             = 0x001000;
        const SPECULAR_TRANSMISSION_UV = 0x002000;
        const THICKNESS_UV             = 0x004000;
        const DIFFUSE_TRANSMISSION_UV  = 0x008000;
        const NORMAL_MAP_UV            = 0x010000;
        const ANISOTROPY_UV            = 0x020000;
        const CLEARCOAT_UV             = 0x040000;
        const CLEARCOAT_ROUGHNESS_UV   = 0x080000;
        const CLEARCOAT_NORMAL_UV      = 0x100000;
        const DEPTH_BIAS               = 0xffffffff_00000000;
    }
}

const STANDARD_MATERIAL_KEY_DEPTH_BIAS_SHIFT: u64 = 32;

impl From<&StandardMaterial> for StandardMaterialKey {
    fn from(material: &StandardMaterial) -> Self {
        let mut key = StandardMaterialKey::empty();
        key.set(
            StandardMaterialKey::CULL_FRONT,
            material.cull_mode == Some(Face::Front),
        );
        key.set(
            StandardMaterialKey::CULL_BACK,
            material.cull_mode == Some(Face::Back),
        );
        key.set(
            StandardMaterialKey::NORMAL_MAP,
            material.normal_map_texture.is_some(),
        );
        key.set(
            StandardMaterialKey::RELIEF_MAPPING,
            matches!(
                material.parallax_mapping_method,
                ParallaxMappingMethod::Relief { .. }
            ),
        );
        key.set(
            StandardMaterialKey::DIFFUSE_TRANSMISSION,
            material.diffuse_transmission > 0.0,
        );
        key.set(
            StandardMaterialKey::SPECULAR_TRANSMISSION,
            material.specular_transmission > 0.0,
        );

        key.set(StandardMaterialKey::CLEARCOAT, material.clearcoat > 0.0);

        #[cfg(feature = "pbr_multi_layer_material_textures")]
        key.set(
            StandardMaterialKey::CLEARCOAT_NORMAL_MAP,
            material.clearcoat > 0.0 && material.clearcoat_normal_texture.is_some(),
        );

        key.set(
            StandardMaterialKey::ANISOTROPY,
            material.anisotropy_strength > 0.0,
        );

        key.set(
            StandardMaterialKey::BASE_COLOR_UV,
            material.base_color_channel != UvChannel::Uv0,
        );

        key.set(
            StandardMaterialKey::EMISSIVE_UV,
            material.emissive_channel != UvChannel::Uv0,
        );
        key.set(
            StandardMaterialKey::METALLIC_ROUGHNESS_UV,
            material.metallic_roughness_channel != UvChannel::Uv0,
        );
        key.set(
            StandardMaterialKey::OCCLUSION_UV,
            material.occlusion_channel != UvChannel::Uv0,
        );
        #[cfg(feature = "pbr_transmission_textures")]
        {
            key.set(
                StandardMaterialKey::SPECULAR_TRANSMISSION_UV,
                material.specular_transmission_channel != UvChannel::Uv0,
            );
            key.set(
                StandardMaterialKey::THICKNESS_UV,
                material.thickness_channel != UvChannel::Uv0,
            );
            key.set(
                StandardMaterialKey::DIFFUSE_TRANSMISSION_UV,
                material.diffuse_transmission_channel != UvChannel::Uv0,
            );
        }

        key.set(
            StandardMaterialKey::NORMAL_MAP_UV,
            material.normal_map_channel != UvChannel::Uv0,
        );

        #[cfg(feature = "pbr_anisotropy_texture")]
        {
            key.set(
                StandardMaterialKey::ANISOTROPY_UV,
                material.anisotropy_channel != UvChannel::Uv0,
            );
        }

        #[cfg(feature = "pbr_multi_layer_material_textures")]
        {
            key.set(
                StandardMaterialKey::CLEARCOAT_UV,
                material.clearcoat_channel != UvChannel::Uv0,
            );
            key.set(
                StandardMaterialKey::CLEARCOAT_ROUGHNESS_UV,
                material.clearcoat_roughness_channel != UvChannel::Uv0,
            );
            key.set(
                StandardMaterialKey::CLEARCOAT_NORMAL_UV,
                material.clearcoat_normal_channel != UvChannel::Uv0,
            );
        }

        key.insert(StandardMaterialKey::from_bits_retain(
            (material.depth_bias as u64) << STANDARD_MATERIAL_KEY_DEPTH_BIAS_SHIFT,
        ));
        key
    }
}

impl Material for StandardMaterial {
    fn fragment_shader() -> ShaderRef {
        PBR_SHADER_HANDLE.into()
    }

    #[inline]
    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }

    #[inline]
    fn opaque_render_method(&self) -> OpaqueRendererMethod {
        match self.opaque_render_method {
            // For now, diffuse transmission doesn't work under deferred rendering as we don't pack
            // the required data into the GBuffer. If this material is set to `Auto`, we report it as
            // `Forward` so that it's rendered correctly, even when the `DefaultOpaqueRendererMethod`
            // is set to `Deferred`.
            //
            // If the developer explicitly sets the `OpaqueRendererMethod` to `Deferred`, we assume
            // they know what they're doing and don't override it.
            OpaqueRendererMethod::Auto if self.diffuse_transmission > 0.0 => {
                OpaqueRendererMethod::Forward
            }
            other => other,
        }
    }

    #[inline]
    fn depth_bias(&self) -> f32 {
        self.depth_bias
    }

    #[inline]
    fn reads_view_transmission_texture(&self) -> bool {
        self.specular_transmission > 0.0
    }

    fn prepass_fragment_shader() -> ShaderRef {
        PBR_PREPASS_SHADER_HANDLE.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        PBR_SHADER_HANDLE.into()
    }

    #[cfg(feature = "meshlet")]
    fn meshlet_mesh_fragment_shader() -> ShaderRef {
        Self::fragment_shader()
    }

    #[cfg(feature = "meshlet")]
    fn meshlet_mesh_prepass_fragment_shader() -> ShaderRef {
        Self::prepass_fragment_shader()
    }

    #[cfg(feature = "meshlet")]
    fn meshlet_mesh_deferred_fragment_shader() -> ShaderRef {
        Self::deferred_fragment_shader()
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(fragment) = descriptor.fragment.as_mut() {
            let shader_defs = &mut fragment.shader_defs;

            for (flags, shader_def) in [
                (
                    StandardMaterialKey::NORMAL_MAP,
                    "STANDARD_MATERIAL_NORMAL_MAP",
                ),
                (StandardMaterialKey::RELIEF_MAPPING, "RELIEF_MAPPING"),
                (
                    StandardMaterialKey::DIFFUSE_TRANSMISSION,
                    "STANDARD_MATERIAL_DIFFUSE_TRANSMISSION",
                ),
                (
                    StandardMaterialKey::SPECULAR_TRANSMISSION,
                    "STANDARD_MATERIAL_SPECULAR_TRANSMISSION",
                ),
                (
                    StandardMaterialKey::DIFFUSE_TRANSMISSION
                        | StandardMaterialKey::SPECULAR_TRANSMISSION,
                    "STANDARD_MATERIAL_DIFFUSE_OR_SPECULAR_TRANSMISSION",
                ),
                (
                    StandardMaterialKey::CLEARCOAT,
                    "STANDARD_MATERIAL_CLEARCOAT",
                ),
                (
                    StandardMaterialKey::CLEARCOAT_NORMAL_MAP,
                    "STANDARD_MATERIAL_CLEARCOAT_NORMAL_MAP",
                ),
                (
                    StandardMaterialKey::ANISOTROPY,
                    "STANDARD_MATERIAL_ANISOTROPY",
                ),
                (
                    StandardMaterialKey::BASE_COLOR_UV,
                    "STANDARD_MATERIAL_BASE_COLOR_UV_B",
                ),
                (
                    StandardMaterialKey::EMISSIVE_UV,
                    "STANDARD_MATERIAL_EMISSIVE_UV_B",
                ),
                (
                    StandardMaterialKey::METALLIC_ROUGHNESS_UV,
                    "STANDARD_MATERIAL_METALLIC_ROUGHNESS_UV_B",
                ),
                (
                    StandardMaterialKey::OCCLUSION_UV,
                    "STANDARD_MATERIAL_OCCLUSION_UV_B",
                ),
                (
                    StandardMaterialKey::SPECULAR_TRANSMISSION_UV,
                    "STANDARD_MATERIAL_SPECULAR_TRANSMISSION_UV_B",
                ),
                (
                    StandardMaterialKey::THICKNESS_UV,
                    "STANDARD_MATERIAL_THICKNESS_UV_B",
                ),
                (
                    StandardMaterialKey::DIFFUSE_TRANSMISSION_UV,
                    "STANDARD_MATERIAL_DIFFUSE_TRANSMISSION_UV_B",
                ),
                (
                    StandardMaterialKey::NORMAL_MAP_UV,
                    "STANDARD_MATERIAL_NORMAL_MAP_UV_B",
                ),
                (
                    StandardMaterialKey::CLEARCOAT_UV,
                    "STANDARD_MATERIAL_CLEARCOAT_UV_B",
                ),
                (
                    StandardMaterialKey::CLEARCOAT_ROUGHNESS_UV,
                    "STANDARD_MATERIAL_CLEARCOAT_ROUGHNESS_UV_B",
                ),
                (
                    StandardMaterialKey::CLEARCOAT_NORMAL_UV,
                    "STANDARD_MATERIAL_CLEARCOAT_NORMAL_UV_B",
                ),
                (
                    StandardMaterialKey::ANISOTROPY_UV,
                    "STANDARD_MATERIAL_ANISOTROPY_UV",
                ),
            ] {
                if key.bind_group_data.intersects(flags) {
                    shader_defs.push(shader_def.into());
                }
            }
        }

        descriptor.primitive.cull_mode = if key
            .bind_group_data
            .contains(StandardMaterialKey::CULL_FRONT)
        {
            Some(Face::Front)
        } else if key.bind_group_data.contains(StandardMaterialKey::CULL_BACK) {
            Some(Face::Back)
        } else {
            None
        };

        if let Some(label) = &mut descriptor.label {
            *label = format!("pbr_{}", *label).into();
        }
        if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
            depth_stencil.bias.constant =
                (key.bind_group_data.bits() >> STANDARD_MATERIAL_KEY_DEPTH_BIAS_SHIFT) as i32;
        }
        Ok(())
    }
}
