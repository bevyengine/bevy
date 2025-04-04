#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

use bevy_app::Plugin;
use contrast_adaptive_sharpening::CasPlugin;
use fxaa::FxaaPlugin;
use smaa::SmaaPlugin;

pub mod contrast_adaptive_sharpening;
pub mod experimental;
pub mod fxaa;
pub mod smaa;

mod taa;

#[derive(Default)]
pub struct AntiAliasingPlugin;
impl Plugin for AntiAliasingPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins((FxaaPlugin, CasPlugin, SmaaPlugin));
    }
}
