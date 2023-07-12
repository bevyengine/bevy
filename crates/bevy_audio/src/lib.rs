//! Audio support for the game engine Bevy
//!
//! ```no_run
//! # use bevy_ecs::prelude::*;
//! # use bevy_audio::{AudioBundle, AudioPlugin, PlaybackSettings};
//! # use bevy_asset::{AssetPlugin, AssetServer};
//! # use bevy_app::{App, AppExit, NoopPluginGroup as MinimalPlugins, Startup};
//! fn main() {
//!    App::new()
//!         .add_plugins((MinimalPlugins, AssetPlugin::default(), AudioPlugin::default()))
//!         .add_systems(Startup, play_background_audio)
//!         .run();
//! }
//!
//! fn play_background_audio(asset_server: Res<AssetServer>, mut commands: Commands) {
//!     commands.spawn(AudioBundle {
//!         source: asset_server.load("background_audio.ogg"),
//!         settings: PlaybackSettings::LOOP,
//!     });
//! }
//! ```

#![forbid(unsafe_code)]
#![allow(clippy::type_complexity)]
#![warn(missing_docs)]

mod audio;
mod audio_output;
mod audio_source;
mod sinks;

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        AudioBundle, AudioSink, AudioSinkPlayback, AudioSource, AudioSourceBundle, Decodable,
        GlobalVolume, PlaybackSettings, SpatialAudioBundle, SpatialAudioSink,
        SpatialAudioSourceBundle, SpatialSettings,
    };
}

pub use audio::*;
pub use audio_source::*;

pub use rodio::cpal::Sample as CpalSample;
pub use rodio::source::Source;
pub use rodio::Sample;
pub use sinks::*;

use bevy_app::prelude::*;
use bevy_asset::{AddAsset, Asset};
use bevy_ecs::prelude::*;

use audio_output::*;

/// Set for the audio playback systems, so they can share a run condition
#[derive(SystemSet, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
struct AudioPlaySet;

/// Adds support for audio playback to a Bevy Application
///
/// Insert an [`AudioBundle`] or [`SpatialAudioBundle`] onto your entities to play audio.
#[derive(Default)]
pub struct AudioPlugin {
    /// The global volume for all audio entities with a [`Volume::Relative`] volume.
    pub global_volume: GlobalVolume,
}

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.global_volume)
            .configure_set(PostUpdate, AudioPlaySet.run_if(audio_output_available))
            .init_resource::<AudioOutput>();

        #[cfg(any(feature = "mp3", feature = "flac", feature = "wav", feature = "vorbis"))]
        {
            app.add_audio_source::<AudioSource>();
            app.init_asset_loader::<AudioLoader>();
        }
    }
}

impl AddAudioSource for App {
    fn add_audio_source<T>(&mut self) -> &mut Self
    where
        T: Decodable + Asset,
        f32: rodio::cpal::FromSample<T::DecoderItem>,
    {
        self.add_asset::<T>().add_systems(
            PostUpdate,
            play_queued_audio_system::<T>.in_set(AudioPlaySet),
        );
        self.add_systems(PostUpdate, cleanup_finished_audio::<T>.in_set(AudioPlaySet));
        self
    }
}
