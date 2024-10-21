use std::path::{Path, PathBuf};

use gltf::texture::{MagFilter, MinFilter, WrappingMode};

use bevy_asset::{Handle, LoadContext, RenderAssetUsages};
use bevy_image::{
    CompressedImageFormats, Image, ImageAddressMode, ImageFilterMode, ImageLoaderSettings,
    ImageSampler, ImageSamplerDescriptor, ImageType,
};
use bevy_utils::HashSet;

#[cfg(not(target_arch = "wasm32"))]
use {bevy_tasks::IoTaskPool, bevy_utils::tracing::warn};

use crate::{data_uri::DataUri, GltfError, GltfLoader, GltfLoaderSettings};

use super::GltfAssetLabel;

/// A texture define in a [`glTF`](gltf::Gltf).
pub struct GltfTexture;

impl GltfTexture {
    /// Load all texture of a [`glTF`](gltf::Gltf)
    pub(crate) async fn load_textures<'a>(
        loader: &GltfLoader,
        load_context: &mut LoadContext<'a>,
        settings: &GltfLoaderSettings,
        gltf: &gltf::Gltf,
        buffer_data: &[Vec<u8>],
        used_textures: &HashSet<usize>,
    ) -> Result<Vec<Handle<Image>>, GltfError> {
        #[cfg(target_arch = "wasm32")]
        let textures = Self::singlethreaded_texture_load(
            load_context,
            loader,
            settings,
            gltf,
            buffer_data,
            used_textures,
        )
        .await;

        #[cfg(not(target_arch = "wasm32"))]
        let textures = {
            if gltf.textures().len() == 1 {
                Self::singlethreaded_texture_load(
                    load_context,
                    loader,
                    settings,
                    gltf,
                    buffer_data,
                    used_textures,
                )
                .await
            } else {
                Ok(Self::multithreaded_texture_load(
                    load_context,
                    loader,
                    settings,
                    gltf,
                    buffer_data,
                    used_textures,
                )
                .await)
            }
        };

        textures
    }

    async fn singlethreaded_texture_load(
        load_context: &mut LoadContext<'_>,
        loader: &GltfLoader,
        settings: &GltfLoaderSettings,
        gltf: &gltf::Gltf,
        buffer_data: &[Vec<u8>],
        used_textures: &HashSet<usize>,
    ) -> Result<Vec<Handle<Image>>, GltfError> {
        let mut textures = vec![];
        for texture in gltf.textures() {
            let parent_path = load_context.path().parent().unwrap();
            let image = Self::load_image(
                texture,
                buffer_data,
                used_textures,
                parent_path,
                loader.supported_compressed_formats,
                settings.load_materials,
            )
            .await?;
            textures.push(Self::process_loaded_texture(load_context, image));
        }
        Ok(textures)
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn multithreaded_texture_load(
        load_context: &mut LoadContext<'_>,
        loader: &GltfLoader,
        settings: &GltfLoaderSettings,
        gltf: &gltf::Gltf,
        buffer_data: &[Vec<u8>],
        used_textures: &HashSet<usize>,
    ) -> Vec<Handle<Image>> {
        IoTaskPool::get()
            .scope(|scope| {
                gltf.textures().for_each(|gltf_texture| {
                    let parent_path = load_context.path().parent().unwrap();
                    let linear_textures = &used_textures;
                    let buffer_data = &buffer_data;
                    scope.spawn(async move {
                        Self::load_image(
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
            .flat_map(|result| match result {
                Ok(image) => Some(Self::process_loaded_texture(load_context, image)),
                Err(err) => {
                    warn!("Error loading glTF texture: {}", err);
                    None
                }
            })
            .collect::<Vec<_>>()
    }

    /// Loads a glTF texture as a bevy [`Image`] and returns it together with its label.
    async fn load_image<'a, 'b>(
        gltf_texture: gltf::Texture<'a>,
        buffer_data: &[Vec<u8>],
        linear_textures: &HashSet<usize>,
        parent_path: &'b Path,
        supported_compressed_formats: CompressedImageFormats,
        render_asset_usages: RenderAssetUsages,
    ) -> Result<ImageOrPath, GltfError> {
        let is_srgb = !linear_textures.contains(&gltf_texture.index());
        let sampler_descriptor = Self::texture_sampler(&gltf_texture);
        #[cfg(all(debug_assertions, feature = "dds"))]
        let name = gltf_texture
            .name()
            .map_or("Unknown GLTF Texture".to_string(), ToString::to_string);
        match gltf_texture.source().source() {
            gltf::image::Source::View { view, mime_type } => {
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
            gltf::image::Source::Uri { uri, mime_type } => {
                let uri = percent_encoding::percent_decode_str(uri)
                    .decode_utf8()
                    .unwrap();
                let uri = uri.as_ref();
                if let Ok(data_uri) = DataUri::parse(uri) {
                    let bytes = data_uri.decode()?;
                    let image_type = ImageType::MimeType(data_uri.mime_type);
                    let image = Image::from_buffer(
                        #[cfg(all(debug_assertions, feature = "dds"))]
                        name,
                        &bytes,
                        mime_type.map(ImageType::MimeType).unwrap_or(image_type),
                        supported_compressed_formats,
                        is_srgb,
                        ImageSampler::Descriptor(sampler_descriptor),
                        render_asset_usages,
                    )?;
                    Ok(ImageOrPath::Image {
                        image,
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

    // TODO: use the threaded impl on wasm once wasm thread pool doesn't deadlock on it
    // See https://github.com/bevyengine/bevy/issues/1924 for more details
    // The taskpool use is also avoided when there is only one texture for performance reasons and
    // to avoid https://github.com/bevyengine/bevy/pull/2725
    // PERF: could this be a Vec instead? Are gltf texture indices dense?
    fn process_loaded_texture(
        load_context: &mut LoadContext,
        texture: ImageOrPath,
    ) -> Handle<Image> {
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
        handle
    }

    /// Extracts the texture sampler data from the glTF texture.
    fn texture_sampler(texture: &gltf::Texture) -> ImageSamplerDescriptor {
        let gltf_sampler = texture.sampler();

        ImageSamplerDescriptor {
            address_mode_u: Self::texture_address_mode(&gltf_sampler.wrap_s()),
            address_mode_v: Self::texture_address_mode(&gltf_sampler.wrap_t()),

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
}

enum ImageOrPath {
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
