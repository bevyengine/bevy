mod audio_output;
mod audio_source;

pub use audio_output::*;
pub use audio_source::*;

pub mod prelude {
    pub use crate::{AudioOutput, AudioSource, Decodable};
}

use bevy_app::prelude::*;
use bevy_asset::AddAsset;
use bevy_ecs::IntoQuerySystem;

/// Adds support for audio playback to an App
#[derive(Default)]
pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<AudioOutput<AudioSource>>()
            .add_asset::<AudioSource>()
            .add_asset_loader::<AudioSource, Mp3Loader>()
            .add_system_to_stage(
                stage::POST_UPDATE,
                play_queued_audio_system::<AudioSource>.system(),
            );
    }
}
