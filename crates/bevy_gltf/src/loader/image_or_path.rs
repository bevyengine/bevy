use std::path::PathBuf;

use bevy_asset::{Handle, LoadContext};
use bevy_image::{Image, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};

use crate::GltfAssetLabel;

pub(crate) enum ImageOrPath {
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

impl ImageOrPath {
    // TODO: use the threaded impl on wasm once wasm thread pool doesn't deadlock on it
    // See https://github.com/bevyengine/bevy/issues/1924 for more details
    // The taskpool use is also avoided when there is only one texture for performance reasons and
    // to avoid https://github.com/bevyengine/bevy/pull/2725
    // PERF: could this be a Vec instead? Are gltf texture indices dense?
    pub(crate) fn process_loaded_texture(
        self,
        load_context: &mut LoadContext,
        handles: &mut Vec<Handle<Image>>,
    ) {
        let handle = match self {
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
}
