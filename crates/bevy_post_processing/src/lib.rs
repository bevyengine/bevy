#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

use bevy_app::Plugin;
use bloom::BloomPlugin;
use dof::DepthOfFieldPlugin;
use motion_blur::MotionBlurPlugin;

pub mod auto_exposure;
pub mod bloom;
pub mod dof;
pub mod motion_blur;
pub mod post_process;

#[derive(Default)]
pub struct PostProcessingPlugin;
impl Plugin for PostProcessingPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins((
            BloomPlugin,
            MotionBlurPlugin,
            DepthOfFieldPlugin,
            post_process::PostProcessingPlugin,
        ));
    }
}
