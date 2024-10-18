mod anisotropy_extension;
mod clearcoat_extension;

use std::path::{Path, PathBuf};

use serde_json::value;

use gltf::{
    image::Source,
    json,
    texture::{Info, MagFilter, MinFilter, TextureTransform, WrappingMode},
    Document, Material,
};

use bevy_asset::{Handle, LoadContext};
use bevy_color::{Color, LinearRgba};
use bevy_math::Affine2;
use bevy_pbr::{StandardMaterial, UvChannel};
use bevy_render::{
    alpha::AlphaMode,
    render_asset::RenderAssetUsages,
    render_resource::Face,
    texture::{
        CompressedImageFormats, Image, ImageAddressMode, ImageFilterMode, ImageLoaderSettings,
        ImageSampler, ImageSamplerDescriptor, ImageType,
    },
};
#[cfg(not(target_arch = "wasm32"))]
use bevy_tasks::IoTaskPool;
use bevy_utils::{tracing::warn, HashMap, HashSet};

use crate::GltfAssetLabel;

use super::{DataUri, GltfError, GltfLoader, GltfLoaderSettings};

pub enum ImageOrPath {
    Image {
        image: Image,
        label: GltfAssetLabel,
    },
    Path {
        path: PathBuf,
        is_srgb: bool,
        sampler_descriptor: ImageSamplerDescriptor,
    },
}

pub fn load_linear_textures(gltf: &gltf::Gltf) -> HashSet<usize> {
    let mut linear_textures = HashSet::default();

    for material in gltf.materials() {
        if let Some(texture) = material.normal_texture() {
            linear_textures.insert(texture.texture().index());
        }
        if let Some(texture) = material.occlusion_texture() {
            linear_textures.insert(texture.texture().index());
        }
        if let Some(texture) = material
            .pbr_metallic_roughness()
            .metallic_roughness_texture()
        {
            linear_textures.insert(texture.texture().index());
        }
        if let Some(texture_index) = material_extension_texture_index(
            &material,
            "KHR_materials_anisotropy",
            "anisotropyTexture",
        ) {
            linear_textures.insert(texture_index);
        }

        // None of the clearcoat maps should be loaded as sRGB.
        #[cfg(feature = "pbr_multi_layer_material_textures")]
        for texture_field_name in [
            "clearcoatTexture",
            "clearcoatRoughnessTexture",
            "clearcoatNormalTexture",
        ] {
            if let Some(texture_index) = material_extension_texture_index(
                &material,
                "KHR_materials_clearcoat",
                texture_field_name,
            ) {
                linear_textures.insert(texture_index);
            }
        }
    }

    linear_textures
}

/// Loads a glTF texture as a bevy [`Image`] and returns it together with its label.
pub async fn load_image<'a, 'b>(
    gltf_texture: gltf::Texture<'a>,
    buffer_data: &[Vec<u8>],
    linear_textures: &HashSet<usize>,
    parent_path: &'b Path,
    supported_compressed_formats: CompressedImageFormats,
    render_asset_usages: RenderAssetUsages,
) -> Result<ImageOrPath, GltfError> {
    let is_srgb = !linear_textures.contains(&gltf_texture.index());
    let sampler_descriptor = texture_sampler(&gltf_texture);
    #[cfg(all(debug_assertions, feature = "dds"))]
    let name = gltf_texture
        .name()
        .map_or("Unknown GLTF Texture".to_string(), ToString::to_string);
    match gltf_texture.source().source() {
        Source::View { view, mime_type } => {
            let start = view.offset();
            let end = view.offset() + view.length();
            let buffer = &buffer_data[view.buffer().index()][start..end];
            let image = Image::from_buffer(
                #[cfg(all(debug_assertions, feature = "dds"))]
                name,
                buffer,
                ImageType::MimeType(mime_type),
                supported_compressed_formats,
                is_srgb,
                ImageSampler::Descriptor(sampler_descriptor),
                render_asset_usages,
            )?;
            Ok(ImageOrPath::Image {
                image,
                label: GltfAssetLabel::Texture(gltf_texture.index()),
            })
        }
        Source::Uri { uri, mime_type } => {
            let uri = percent_encoding::percent_decode_str(uri)
                .decode_utf8()
                .unwrap();
            let uri = uri.as_ref();
            if let Ok(data_uri) = DataUri::parse(uri) {
                let bytes = data_uri.decode()?;
                let image_type = ImageType::MimeType(data_uri.mime_type);
                Ok(ImageOrPath::Image {
                    image: Image::from_buffer(
                        #[cfg(all(debug_assertions, feature = "dds"))]
                        name,
                        &bytes,
                        mime_type.map(ImageType::MimeType).unwrap_or(image_type),
                        supported_compressed_formats,
                        is_srgb,
                        ImageSampler::Descriptor(sampler_descriptor),
                        render_asset_usages,
                    )?,
                    label: GltfAssetLabel::Texture(gltf_texture.index()),
                })
            } else {
                let image_path = parent_path.join(uri);
                Ok(ImageOrPath::Path {
                    path: image_path,
                    is_srgb,
                    sampler_descriptor,
                })
            }
        }
    }
}

pub async fn collect_texture_handles<'a>(
    loader: &GltfLoader,
    load_context: &mut LoadContext<'a>,
    settings: &GltfLoaderSettings,
    gltf: &gltf::Gltf,
    buffer_data: &[Vec<u8>],
    linear_textures: &HashSet<usize>,
) -> Result<Vec<Handle<Image>>, GltfError> {
    let mut texture_handles = vec![];
    if gltf.textures().len() == 1 || cfg!(target_arch = "wasm32") {
        for texture in gltf.textures() {
            let parent_path = load_context.path().parent().unwrap();
            let image = load_image(
                texture,
                buffer_data,
                linear_textures,
                parent_path,
                loader.supported_compressed_formats,
                settings.load_materials,
            )
            .await?;
            process_loaded_texture(load_context, &mut texture_handles, image);
        }
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        IoTaskPool::get()
            .scope(|scope| {
                gltf.textures().for_each(|gltf_texture| {
                    let parent_path = load_context.path().parent().unwrap();
                    let linear_textures = &linear_textures;
                    let buffer_data = &buffer_data;
                    scope.spawn(async move {
                        load_image(
                            gltf_texture,
                            buffer_data,
                            linear_textures,
                            parent_path,
                            loader.supported_compressed_formats,
                            settings.load_materials,
                        )
                        .await
                    });
                });
            })
            .into_iter()
            .for_each(|result| match result {
                Ok(image) => {
                    process_loaded_texture(load_context, &mut texture_handles, image);
                }
                Err(err) => {
                    warn!("Error loading glTF texture: {}", err);
                }
            });
    }
    Ok(texture_handles)
}

pub fn load_materials(
    load_context: &mut LoadContext,
    settings: &GltfLoaderSettings,
    gltf: &gltf::Gltf,
) -> (
    Vec<Handle<StandardMaterial>>,
    HashMap<Box<str>, Handle<StandardMaterial>>,
) {
    let mut materials = vec![];
    let mut named_materials = HashMap::default();

    // Only include materials in the output if they're set to be retained in the MAIN_WORLD and/or RENDER_WORLD by the load_materials flag
    if !settings.load_materials.is_empty() {
        // NOTE: materials must be loaded after textures because image load() calls will
        // happen before load_with_settings, preventing is_srgb from being set properly
        for material in gltf.materials() {
            let handle = load_material(&material, load_context, &gltf.document, false);
            if let Some(name) = material.name() {
                named_materials.insert(name.into(), handle.clone());
            }
            materials.push(handle);
        }
    }

    (materials, named_materials)
}

/// Loads a glTF material as a bevy [`StandardMaterial`] and returns it.
pub fn load_material(
    material: &Material,
    load_context: &mut LoadContext,
    document: &Document,
    is_scale_inverted: bool,
) -> Handle<StandardMaterial> {
    let material_label = material_label(material, is_scale_inverted);
    load_context.labeled_asset_scope(material_label, |load_context| {
        let pbr = material.pbr_metallic_roughness();

        // TODO: handle missing label handle errors here?
        let color = pbr.base_color_factor();
        let base_color_channel = pbr
            .base_color_texture()
            .map(|info| get_uv_channel(material, "base color", info.tex_coord()))
            .unwrap_or_default();
        let base_color_texture = pbr
            .base_color_texture()
            .map(|info| texture_handle(load_context, &info.texture()));

        let uv_transform = pbr
            .base_color_texture()
            .and_then(|info| {
                info.texture_transform()
                    .map(convert_texture_transform_to_affine2)
            })
            .unwrap_or_default();

        let normal_map_channel = material
            .normal_texture()
            .map(|info| get_uv_channel(material, "normal map", info.tex_coord()))
            .unwrap_or_default();
        let normal_map_texture: Option<Handle<Image>> =
            material.normal_texture().map(|normal_texture| {
                // TODO: handle normal_texture.scale
                texture_handle(load_context, &normal_texture.texture())
            });

        let metallic_roughness_channel = pbr
            .metallic_roughness_texture()
            .map(|info| get_uv_channel(material, "metallic/roughness", info.tex_coord()))
            .unwrap_or_default();
        let metallic_roughness_texture = pbr.metallic_roughness_texture().map(|info| {
            warn_on_differing_texture_transforms(
                material,
                &info,
                uv_transform,
                "metallic/roughness",
            );
            texture_handle(load_context, &info.texture())
        });

        let occlusion_channel = material
            .occlusion_texture()
            .map(|info| get_uv_channel(material, "occlusion", info.tex_coord()))
            .unwrap_or_default();
        let occlusion_texture = material.occlusion_texture().map(|occlusion_texture| {
            // TODO: handle occlusion_texture.strength() (a scalar multiplier for occlusion strength)
            texture_handle(load_context, &occlusion_texture.texture())
        });

        let emissive = material.emissive_factor();
        let emissive_channel = material
            .emissive_texture()
            .map(|info| get_uv_channel(material, "emissive", info.tex_coord()))
            .unwrap_or_default();
        let emissive_texture = material.emissive_texture().map(|info| {
            // TODO: handle occlusion_texture.strength() (a scalar multiplier for occlusion strength)
            warn_on_differing_texture_transforms(material, &info, uv_transform, "emissive");
            texture_handle(load_context, &info.texture())
        });

        #[cfg(feature = "pbr_transmission_textures")]
        let (specular_transmission, specular_transmission_channel, specular_transmission_texture) =
            material
                .transmission()
                .map_or((0.0, UvChannel::Uv0, None), |transmission| {
                    let specular_transmission_channel = transmission
                        .transmission_texture()
                        .map(|info| {
                            get_uv_channel(material, "specular/transmission", info.tex_coord())
                        })
                        .unwrap_or_default();
                    let transmission_texture: Option<Handle<Image>> = transmission
                        .transmission_texture()
                        .map(|transmission_texture| {
                            texture_handle(load_context, &transmission_texture.texture())
                        });

                    (
                        transmission.transmission_factor(),
                        specular_transmission_channel,
                        transmission_texture,
                    )
                });

        #[cfg(not(feature = "pbr_transmission_textures"))]
        let specular_transmission = material
            .transmission()
            .map_or(0.0, |transmission| transmission.transmission_factor());

        #[cfg(feature = "pbr_transmission_textures")]
        let (
            thickness,
            thickness_channel,
            thickness_texture,
            attenuation_distance,
            attenuation_color,
        ) = material.volume().map_or(
            (0.0, UvChannel::Uv0, None, f32::INFINITY, [1.0, 1.0, 1.0]),
            |volume| {
                let thickness_channel = volume
                    .thickness_texture()
                    .map(|info| get_uv_channel(material, "thickness", info.tex_coord()))
                    .unwrap_or_default();
                let thickness_texture: Option<Handle<Image>> =
                    volume.thickness_texture().map(|thickness_texture| {
                        texture_handle(load_context, &thickness_texture.texture())
                    });

                (
                    volume.thickness_factor(),
                    thickness_channel,
                    thickness_texture,
                    volume.attenuation_distance(),
                    volume.attenuation_color(),
                )
            },
        );

        #[cfg(not(feature = "pbr_transmission_textures"))]
        let (thickness, attenuation_distance, attenuation_color) =
            material
                .volume()
                .map_or((0.0, f32::INFINITY, [1.0, 1.0, 1.0]), |volume| {
                    (
                        volume.thickness_factor(),
                        volume.attenuation_distance(),
                        volume.attenuation_color(),
                    )
                });

        let ior = material.ior().unwrap_or(1.5);

        // Parse the `KHR_materials_clearcoat` extension data if necessary.
        let clearcoat =
            clearcoat_extension::ClearcoatExtension::parse(load_context, document, material)
                .unwrap_or_default();

        // Parse the `KHR_materials_anisotropy` extension data if necessary.
        let anisotropy =
            anisotropy_extension::AnisotropyExtension::parse(load_context, document, material)
                .unwrap_or_default();

        // We need to operate in the Linear color space and be willing to exceed 1.0 in our channels
        let base_emissive = LinearRgba::rgb(emissive[0], emissive[1], emissive[2]);
        let emissive = base_emissive * material.emissive_strength().unwrap_or(1.0);

        StandardMaterial {
            base_color: Color::linear_rgba(color[0], color[1], color[2], color[3]),
            base_color_channel,
            base_color_texture,
            perceptual_roughness: pbr.roughness_factor(),
            metallic: pbr.metallic_factor(),
            metallic_roughness_channel,
            metallic_roughness_texture,
            normal_map_channel,
            normal_map_texture,
            double_sided: material.double_sided(),
            cull_mode: if material.double_sided() {
                None
            } else if is_scale_inverted {
                Some(Face::Front)
            } else {
                Some(Face::Back)
            },
            occlusion_channel,
            occlusion_texture,
            emissive,
            emissive_channel,
            emissive_texture,
            specular_transmission,
            #[cfg(feature = "pbr_transmission_textures")]
            specular_transmission_channel,
            #[cfg(feature = "pbr_transmission_textures")]
            specular_transmission_texture,
            thickness,
            #[cfg(feature = "pbr_transmission_textures")]
            thickness_channel,
            #[cfg(feature = "pbr_transmission_textures")]
            thickness_texture,
            ior,
            attenuation_distance,
            attenuation_color: Color::linear_rgb(
                attenuation_color[0],
                attenuation_color[1],
                attenuation_color[2],
            ),
            unlit: material.unlit(),
            alpha_mode: alpha_mode(material),
            uv_transform,
            clearcoat: clearcoat.clearcoat_factor.unwrap_or_default() as f32,
            clearcoat_perceptual_roughness: clearcoat.clearcoat_roughness_factor.unwrap_or_default()
                as f32,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_channel: clearcoat.clearcoat_channel,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_texture: clearcoat.clearcoat_texture,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_roughness_channel: clearcoat.clearcoat_roughness_channel,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_roughness_texture: clearcoat.clearcoat_roughness_texture,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_normal_channel: clearcoat.clearcoat_normal_channel,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_normal_texture: clearcoat.clearcoat_normal_texture,
            anisotropy_strength: anisotropy.anisotropy_strength.unwrap_or_default() as f32,
            anisotropy_rotation: anisotropy.anisotropy_rotation.unwrap_or_default() as f32,
            #[cfg(feature = "pbr_anisotropy_texture")]
            anisotropy_channel: anisotropy.anisotropy_channel,
            #[cfg(feature = "pbr_anisotropy_texture")]
            anisotropy_texture: anisotropy.anisotropy_texture,
            ..Default::default()
        }
    })
}

// TODO: use the threaded impl on wasm once wasm thread pool doesn't deadlock on it
// See https://github.com/bevyengine/bevy/issues/1924 for more details
// The taskpool use is also avoided when there is only one texture for performance reasons and
// to avoid https://github.com/bevyengine/bevy/pull/2725
// PERF: could this be a Vec instead? Are gltf texture indices dense?
fn process_loaded_texture(
    load_context: &mut LoadContext,
    handles: &mut Vec<Handle<Image>>,
    texture: ImageOrPath,
) {
    let handle = match texture {
        ImageOrPath::Image { label, image } => {
            load_context.add_labeled_asset(label.to_string(), image)
        }
        ImageOrPath::Path {
            path,
            is_srgb,
            sampler_descriptor,
        } => load_context
            .loader()
            .with_settings(move |settings: &mut ImageLoaderSettings| {
                settings.is_srgb = is_srgb;
                settings.sampler = ImageSampler::Descriptor(sampler_descriptor.clone());
            })
            .load(path),
    };
    handles.push(handle);
}

/// Extracts the texture sampler data from the glTF texture.
fn texture_sampler(texture: &gltf::Texture) -> ImageSamplerDescriptor {
    let gltf_sampler = texture.sampler();

    ImageSamplerDescriptor {
        address_mode_u: texture_address_mode(&gltf_sampler.wrap_s()),
        address_mode_v: texture_address_mode(&gltf_sampler.wrap_t()),

        mag_filter: gltf_sampler
            .mag_filter()
            .map(|mf| match mf {
                MagFilter::Nearest => ImageFilterMode::Nearest,
                MagFilter::Linear => ImageFilterMode::Linear,
            })
            .unwrap_or(ImageSamplerDescriptor::default().mag_filter),

        min_filter: gltf_sampler
            .min_filter()
            .map(|mf| match mf {
                MinFilter::Nearest
                | MinFilter::NearestMipmapNearest
                | MinFilter::NearestMipmapLinear => ImageFilterMode::Nearest,
                MinFilter::Linear
                | MinFilter::LinearMipmapNearest
                | MinFilter::LinearMipmapLinear => ImageFilterMode::Linear,
            })
            .unwrap_or(ImageSamplerDescriptor::default().min_filter),

        mipmap_filter: gltf_sampler
            .min_filter()
            .map(|mf| match mf {
                MinFilter::Nearest
                | MinFilter::Linear
                | MinFilter::NearestMipmapNearest
                | MinFilter::LinearMipmapNearest => ImageFilterMode::Nearest,
                MinFilter::NearestMipmapLinear | MinFilter::LinearMipmapLinear => {
                    ImageFilterMode::Linear
                }
            })
            .unwrap_or(ImageSamplerDescriptor::default().mipmap_filter),

        ..Default::default()
    }
}

/// Maps the texture address mode form glTF to wgpu.
fn texture_address_mode(gltf_address_mode: &WrappingMode) -> ImageAddressMode {
    match gltf_address_mode {
        WrappingMode::ClampToEdge => ImageAddressMode::ClampToEdge,
        WrappingMode::Repeat => ImageAddressMode::Repeat,
        WrappingMode::MirroredRepeat => ImageAddressMode::MirrorRepeat,
    }
}

fn texture_handle(load_context: &mut LoadContext, texture: &gltf::Texture) -> Handle<Image> {
    match texture.source().source() {
        Source::View { .. } => {
            load_context.get_label_handle(GltfAssetLabel::Texture(texture.index()).to_string())
        }
        Source::Uri { uri, .. } => {
            let uri = percent_encoding::percent_decode_str(uri)
                .decode_utf8()
                .unwrap();
            let uri = uri.as_ref();
            if let Ok(_data_uri) = DataUri::parse(uri) {
                load_context.get_label_handle(GltfAssetLabel::Texture(texture.index()).to_string())
            } else {
                let parent = load_context.path().parent().unwrap();
                let image_path = parent.join(uri);
                load_context.load(image_path)
            }
        }
    }
}

/// Returns the index (within the `textures` array) of the texture with the
/// given field name in the data for the material extension with the given name,
/// if there is one.
fn material_extension_texture_index(
    material: &Material,
    extension_name: &str,
    texture_field_name: &str,
) -> Option<usize> {
    Some(
        value::from_value::<json::texture::Info>(
            material
                .extensions()?
                .get(extension_name)?
                .as_object()?
                .get(texture_field_name)?
                .clone(),
        )
        .ok()?
        .index
        .value(),
    )
}

/// Returns the label for the `material`.
pub fn material_label(material: &Material, is_scale_inverted: bool) -> String {
    if let Some(index) = material.index() {
        GltfAssetLabel::Material {
            index,
            is_scale_inverted,
        }
        .to_string()
    } else {
        GltfAssetLabel::DefaultMaterial.to_string()
    }
}

/// Returns true if the material needs mesh tangents in order to be successfully
/// rendered.
///
/// We generate them if this function returns true.
pub fn material_needs_tangents(material: &Material) -> bool {
    if material.normal_texture().is_some() {
        return true;
    }

    #[cfg(feature = "pbr_multi_layer_material_textures")]
    if material_extension_texture_index(
        material,
        "KHR_materials_clearcoat",
        "clearcoatNormalTexture",
    )
    .is_some()
    {
        return true;
    }

    false
}

fn get_uv_channel(material: &Material, texture_kind: &str, tex_coord: u32) -> UvChannel {
    match tex_coord {
        0 => UvChannel::Uv0,
        1 => UvChannel::Uv1,
        _ => {
            let material_name = material
                .name()
                .map(|n| format!("the material \"{n}\""))
                .unwrap_or_else(|| "an unnamed material".to_string());
            let material_index = material
                .index()
                .map(|i| format!("index {i}"))
                .unwrap_or_else(|| "default".to_string());
            warn!(
            "Only 2 UV Channels are supported, but {material_name} ({material_index}) \
            has the TEXCOORD attribute {} on texture kind {texture_kind}, which will fallback to 0.",
            tex_coord,
        );
            UvChannel::Uv0
        }
    }
}

fn warn_on_differing_texture_transforms(
    material: &Material,
    info: &Info,
    texture_transform: Affine2,
    texture_kind: &str,
) {
    let has_differing_texture_transform = info
        .texture_transform()
        .map(convert_texture_transform_to_affine2)
        .is_some_and(|t| t != texture_transform);
    if has_differing_texture_transform {
        let material_name = material
            .name()
            .map(|n| format!("the material \"{n}\""))
            .unwrap_or_else(|| "an unnamed material".to_string());
        let texture_name = info
            .texture()
            .name()
            .map(|n| format!("its {texture_kind} texture \"{n}\""))
            .unwrap_or_else(|| format!("its unnamed {texture_kind} texture"));
        let material_index = material
            .index()
            .map(|i| format!("index {i}"))
            .unwrap_or_else(|| "default".to_string());
        warn!(
            "Only texture transforms on base color textures are supported, but {material_name} ({material_index}) \
            has a texture transform on {texture_name} (index {}), which will be ignored.", info.texture().index()
        );
    }
}

fn alpha_mode(material: &Material) -> AlphaMode {
    match material.alpha_mode() {
        gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
        gltf::material::AlphaMode::Mask => AlphaMode::Mask(material.alpha_cutoff().unwrap_or(0.5)),
        gltf::material::AlphaMode::Blend => AlphaMode::Blend,
    }
}

fn convert_texture_transform_to_affine2(texture_transform: TextureTransform) -> Affine2 {
    Affine2::from_scale_angle_translation(
        texture_transform.scale().into(),
        -texture_transform.rotation(),
        texture_transform.offset().into(),
    )
}
