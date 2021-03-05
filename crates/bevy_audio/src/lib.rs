mod audio;
mod audio_output;
mod audio_source;

pub mod prelude {
    pub use crate::{Audio, AudioOutput, AudioSource, Decodable};
}

pub use audio::*;
pub use audio_output::*;
pub use audio_source::*;

use bevy_app::prelude::*;
use bevy_asset::AddAsset;
use bevy_ecs::system::IntoExclusiveSystem;

/// Adds support for audio playback to an App
#[derive(Default)]
pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_non_send_resource::<AudioOutput<AudioSource>>()
            .add_asset::<AudioSource>()
            .init_asset_loader::<Mp3Loader>()
            .init_resource::<Audio<AudioSource>>()
            .add_system_to_stage(
                CoreStage::PostUpdate,
                play_queued_audio_system::<AudioSource>.exclusive_system(),
            );
    }
}
