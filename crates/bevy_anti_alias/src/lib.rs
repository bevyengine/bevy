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

/// A [`SystemSet`] for Anti-aliasing technique systems.
///
/// Nothing prevents a camera from enabling more than one of these, but in practice
/// only one is usually active, and the ordering between them is not meaningful.
///
/// Members should declare `.ambiguous_with(AntiAliasingSystems)` to suppress the
/// ambiguity report. These systems take `&mut ViewTarget` and so are never run
/// concurrently.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AntiAliasingSystems;

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
