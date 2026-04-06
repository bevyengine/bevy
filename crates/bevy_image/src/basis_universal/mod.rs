#[cfg(feature = "basis_universal_saver")]
mod saver;
#[cfg(feature = "basis_universal_saver")]
pub use saver::*;

use crate::{CompressedImageFormats, Image, ImageLoader, TextureError};
use basisu_c_sys::extra::{BasisuTranscoder, SupportedTextureCompression};
use bevy_app::{App, Plugin};
#[cfg(all(
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown",
))]
use bevy_platform::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

/// Converts and transcodes KTX2 Basis Universal bytes to a bevy [`Image`] using the given compressed format support. All basis universal compressed formats (ETC1S, UASTC, ASTC, XUASTC) are supported. Zstd supercompression is always supported. No support for `.basis` files.
///
/// The current integrated basis universl version is 2.10
///
/// Default transcode target selection:
///
/// | BasisU format                  | Target selection                                               |
/// | ------------------------------ | -------------------------------------------------------------- |
/// | ETC1S                          | Bc7Rgba/Bc5Rg/Bc4R > Etc2Rgba8/Etc2Rgb8/EacRg11/EacR11 > Rgba8 |
/// | UASTC_LDR, ASTC_LDR, XUASTC_LDR| Astc > Bc7Rgba > Etc2Rgba8/Etc2Rgb8/EacRg11/EacR11 > Rgba8     |
/// | UASTC_HDR, ASTC_HDR            | Astc > Bc6hRgbUfloat > Rgba16Float                             |
pub fn ktx2_basisu_buffer_to_image(
    buffer: &[u8],
    supported_compressed_formats: CompressedImageFormats,
    is_srgb: bool,
) -> Result<Image, TextureError> {
    let src_bytes = buffer.len();

    let _span = bevy_log::info_span!("Transcoding basisu texture").entered();
    let time = if bevy_log::STATIC_MAX_LEVEL >= bevy_log::Level::DEBUG {
        Some(bevy_platform::time::Instant::now())
    } else {
        None
    };
    let mut compressions = SupportedTextureCompression::empty();
    if supported_compressed_formats.contains(CompressedImageFormats::ASTC_LDR) {
        compressions |= SupportedTextureCompression::ASTC_LDR;
    }
    if supported_compressed_formats.contains(CompressedImageFormats::ASTC_HDR) {
        compressions |= SupportedTextureCompression::ASTC_HDR;
    }
    if supported_compressed_formats.contains(CompressedImageFormats::BC) {
        compressions |= SupportedTextureCompression::BC;
    }
    if supported_compressed_formats.contains(CompressedImageFormats::ETC2) {
        compressions |= SupportedTextureCompression::ETC2;
    }
    let mut transcoder = BasisuTranscoder::new();
    let info = transcoder.prepare(buffer, compressions, basisu_c_sys::extra::ChannelType::Auto)?;

    let out_image = transcoder.transcode(None, Some(is_srgb))?;

    if bevy_log::STATIC_MAX_LEVEL >= bevy_log::Level::DEBUG {
        bevy_log::debug!(
                "Transcoded basisu texture, \
                {:?} -> {:?}, {}kb -> {}kb. \
                Preferred target: {:?}, extents: {:?}, level count: {}, view dimension: {:?} in {:?}",
                info.basis_format,
                out_image.texture_descriptor.format,
                src_bytes as f32 / 1000.0,
                out_image.data.as_ref().unwrap().len() as f32 / 1000.0,
                info.preferred_target,
                out_image.texture_descriptor.size,
                info.levels,
                out_image
                    .texture_view_descriptor
                    .as_ref()
                    .unwrap()
                    .dimension
                    .unwrap(),
                time.unwrap().elapsed(),
            );
    }

    Ok(Image {
        data: out_image.data,
        data_order: out_image.data_order,
        texture_descriptor: out_image.texture_descriptor,
        texture_view_descriptor: out_image.texture_view_descriptor,
        ..Default::default()
    })
}

/// Provides the necessary basis universal initialization.
/// Any bassiu encoding or transcoding will fail before this plugin is initialized.
pub struct BasisUniversalPlugin;

/// Provides basis universal saver and asset processor
#[cfg(feature = "basis_universal_saver")]
pub struct BasisUniversalSaverPlugin {
    /// The file extensions handled by the basisu asset processor.
    ///
    /// Default is [`ImageLoader::SUPPORTED_FILE_EXTENSIONS`] except ktx2 and .dds.
    pub processor_extensions: Vec<String>,
}

impl Default for BasisUniversalSaverPlugin {
    fn default() -> Self {
        Self {
            processor_extensions: ImageLoader::SUPPORTED_FILE_EXTENSIONS
                .iter()
                .filter(|s| !["ktx2", "dds"].contains(s))
                .map(ToString::to_string)
                .collect(),
        }
    }
}

impl Plugin for BasisUniversalSaverPlugin {
    fn build(&self, app: &mut App) {
        if let Some(asset_processor) = app
            .world()
            .get_resource::<bevy_asset::processor::AssetProcessor>()
        {
            asset_processor.register_processor::<BasisuProcessor>(BasisuSaver.into());
            for ext in &self.processor_extensions {
                asset_processor.set_default_processor::<BasisuProcessor>(ext.as_str());
            }
        }
    }
}

#[cfg(all(
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown",
))]
#[derive(bevy_ecs::resource::Resource, Clone)]
struct BasisuWasmReady(Arc<AtomicUsize>);

impl Plugin for BasisUniversalPlugin {
    fn build(&self, _app: &mut App) {
        #[cfg(all(
            target_arch = "wasm32",
            target_vendor = "unknown",
            target_os = "unknown",
        ))]
        {
            let ready = BasisuWasmReady(Arc::new(AtomicUsize::new(0)));
            let r = ready.clone();
            bevy_tasks::IoTaskPool::get()
                .spawn_local(async move {
                    basisu_c_sys::extra::basisu_transcoder_init().await;
                    #[cfg(feature = "basis_universal_saver")]
                    basisu_c_sys::extra::basisu_encoder_init().await;
                    bevy_log::debug!("Basisu wasm initialized");
                    r.0.store(1, Ordering::Release);
                })
                .detach();
            _app.insert_resource(ready);
        }
        #[cfg(not(all(
            target_arch = "wasm32",
            target_vendor = "unknown",
            target_os = "unknown",
        )))]
        {
            bevy_tasks::block_on(basisu_c_sys::extra::basisu_transcoder_init());
            #[cfg(feature = "basis_universal_saver")]
            bevy_tasks::block_on(basisu_c_sys::extra::basisu_encoder_init());
        }
    }

    #[cfg(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown",
    ))]
    fn ready(&self, app: &App) -> bool {
        app.world()
            .resource::<BasisuWasmReady>()
            .0
            .load(Ordering::Acquire)
            != 0
    }

    fn finish(&self, _app: &mut App) {
        #[cfg(all(
            target_arch = "wasm32",
            target_vendor = "unknown",
            target_os = "unknown",
        ))]
        _app.world_mut().remove_resource::<BasisuWasmReady>();
    }
}
