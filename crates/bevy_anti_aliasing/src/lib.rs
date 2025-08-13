#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
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
pub mod fxaa;
pub mod smaa;
pub mod taa;

#[derive(Default)]
pub struct AntiAliasingPlugin;
impl Plugin for AntiAliasingPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins((FxaaPlugin, SmaaPlugin, TemporalAntiAliasPlugin, CasPlugin));
    }
}
