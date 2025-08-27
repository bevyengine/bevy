#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

pub mod auto_exposure;
pub mod bloom;
pub mod dof;
pub mod motion_blur;
pub mod msaa_writeback;
pub mod post_process;

use crate::{
    bloom::BloomPlugin, dof::DepthOfFieldPlugin, motion_blur::MotionBlurPlugin,
    msaa_writeback::MsaaWritebackPlugin, post_process::PostProcessingPlugin,
};
use bevy_app::{App, Plugin};

#[derive(Default)]
pub struct PostProcessPlugin;

impl Plugin for PostProcessPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MsaaWritebackPlugin,
            BloomPlugin,
            MotionBlurPlugin,
            DepthOfFieldPlugin,
            PostProcessingPlugin,
        ));
    }
}
