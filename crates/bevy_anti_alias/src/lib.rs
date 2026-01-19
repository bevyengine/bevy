#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

use bevy_app::Plugin;
use bevy_ecs::schedule::SystemSet;
use contrast_adaptive_sharpening::CasPlugin;
use fxaa::FxaaPlugin;
use smaa::SmaaPlugin;
use taa::TemporalAntiAliasPlugin;

pub mod contrast_adaptive_sharpening;
#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
pub mod dlss;
pub mod fxaa;
pub mod smaa;
pub mod taa;

/// System set for ordering render graph systems relative to any anti-aliasing implementation.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct AntiAliasing;

/// Adds fxaa, smaa, taa, contrast aware sharpening, and optional dlss support.
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
        ));
    }
}
