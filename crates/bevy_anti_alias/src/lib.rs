#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

use bevy_app::Plugin;
use contrast_adaptive_sharpening::CasPlugin;
use fxaa::FxaaPlugin;
use smaa::SmaaPlugin;
use taa::TemporalAntiAliasPlugin;

pub mod contrast_adaptive_sharpening;
#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
pub mod dlss;
pub mod fxaa;
#[cfg(all(
    feature = "metal_fx",
    not(feature = "force_disable_metal_fx"),
    target_vendor = "apple",
))]
pub mod metal_fx;
#[cfg(any(
    all(feature = "dlss", not(feature = "force_disable_dlss")),
    all(feature = "metal_fx", not(feature = "force_disable_metal_fx")),
))]
pub mod ray_reconstruction;
pub mod smaa;
pub mod taa;

/// Adds fxaa, smaa, taa, contrast aware sharpening, and optional dlss / MetalFX support.
#[derive(Default)]
pub struct AntiAliasPlugin;

impl Plugin for AntiAliasPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins((
            FxaaPlugin,
            SmaaPlugin,
            TemporalAntiAliasPlugin,
            CasPlugin,
            #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
            dlss::DlssPlugin,
            #[cfg(all(
                feature = "metal_fx",
                not(feature = "force_disable_metal_fx"),
                target_vendor = "apple",
            ))]
            metal_fx::MetalFxPlugin,
        ));
    }
}
