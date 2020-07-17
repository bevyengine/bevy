mod audio_output;
mod audio_source;

pub use audio_output::*;
pub use audio_source::*;

use bevy_app::prelude::*;
use bevy_asset::AddAsset;
use bevy_ecs::IntoQuerySystem;

#[derive(Default)]
pub struct AudioPlugin;

impl AppPlugin for AudioPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<AudioOutput>()
            .add_asset::<AudioSource>()
            .add_asset_loader::<AudioSource, Mp3Loader>()
            .add_system_to_stage(stage::POST_UPDATE, play_queued_audio_system.system());
    }
}
