#![cfg_attr(docsrs, feature(doc_cfg))]

//! Provides loader and saver for Basis Universal KTX2 textures.
//! See [`loader`] and [`saver`] for more information.
//!
//! This uses [Basis Universal v2.1](https://github.com/BinomialLLC/basis_universal) C++ library. All basis universal formats are supported.

pub mod loader;
#[cfg(feature = "saver")]
pub mod saver;

use bevy_asset::AssetApp;
use bevy_image::{CompressedImageFormatSupport, CompressedImageFormats, ImageLoader};
#[cfg(all(
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown",
))]
use bevy_platform::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use bevy_app::{App, Plugin};

use crate::loader::BasisuLoader;
#[cfg(feature = "saver")]
use crate::saver::{BasisuProcessor, BasisuSaver};

/// Provides basis universal texture loader and saver.
pub struct BasisUniversalPlugin {
    /// The file extensions handled by the basisu asset processor.
    ///
    /// Default is [`ImageLoader::SUPPORTED_FILE_EXTENSIONS`] except ktx2 and .dds.
    pub processor_extensions: Vec<String>,
}

impl Default for BasisUniversalPlugin {
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

#[cfg(all(
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown",
))]
#[derive(Resource, Clone, Deref)]
struct BasisuWasmReady(Arc<AtomicUsize>);

impl Plugin for BasisUniversalPlugin {
    fn build(&self, app: &mut App) {
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
                    #[cfg(feature = "saver")]
                    basisu_c_sys::extra::basisu_encoder_init().await;
                    bevy::log::debug!("Basisu wasm initialized");
                    r.store(1, Ordering::Release);
                })
                .detach();
            app.insert_resource(ready);
        }
        #[cfg(not(all(
            target_arch = "wasm32",
            target_vendor = "unknown",
            target_os = "unknown",
        )))]
        {
            bevy_tasks::block_on(basisu_c_sys::extra::basisu_transcoder_init());
            #[cfg(feature = "saver")]
            bevy_tasks::block_on(basisu_c_sys::extra::basisu_encoder_init());
        }
        app.preregister_asset_loader::<BasisuLoader>(&["basisu.ktx2"]);

        #[cfg(feature = "saver")]
        {
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
    fn ready(&self, app: &App) -> bool {
        app.world()
            .resource::<BasisuWasmReady>()
            .load(Ordering::Acquire)
            != 0
    }

    fn finish(&self, app: &mut App) {
        #[cfg(all(
            target_arch = "wasm32",
            target_vendor = "unknown",
            target_os = "unknown",
        ))]
        app.world_mut().remove_resource::<BasisuWasmReady>();

        let supported_compressed_formats = if let Some(resource) =
            app.world().get_resource::<CompressedImageFormatSupport>()
        {
            resource.0
        } else {
            bevy_log::warn!("CompressedImageFormatSupport resource not found. It should either be initialized in finish() of \
                       RenderPlugin, or manually if not using the RenderPlugin or the WGPU backend.");
            CompressedImageFormats::NONE
        };

        app.register_asset_loader(BasisuLoader::new(supported_compressed_formats));
    }
}
