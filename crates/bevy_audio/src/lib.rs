//! Audio support for the game engine Bevy
//!
//! ```no_run
//! # use bevy_ecs::{system::Res, event::EventWriter};
//! # use bevy_audio::{Audio, AudioPlugin};
//! # use bevy_asset::{AssetPlugin, AssetServer};
//! # use bevy_app::{App, AppExit, NoopPluginGroup as MinimalPlugins};
//! fn main() {
//!    App::new()
//!         .add_plugins(MinimalPlugins)
//!         .add_plugin(AssetPlugin::default())
//!         .add_plugin(AudioPlugin)
//!         .add_startup_system(play_background_audio)
//!         .run();
//! }
//!
//! fn play_background_audio(asset_server: Res<AssetServer>, audio: Res<Audio>) {
//!     audio.play(asset_server.load("background_audio.ogg"));
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod audio;
mod audio_output;
mod audio_source;
mod sinks;

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        Audio, AudioOutput, AudioSink, AudioSinkPlayback, AudioSource, Decodable, PlaybackSettings,
        SpatialAudioSink,
    };
}

pub use audio::*;
pub use audio_output::*;
pub use audio_source::*;

pub use rodio::cpal::Sample as CpalSample;
pub use rodio::source::Source;
pub use rodio::Sample;
pub use sinks::*;

use bevy_app::prelude::*;
use bevy_asset::{AddAsset, Asset};
use bevy_ecs::prelude::*;

/// Adds support for audio playback to a Bevy Application
///
/// Use the [`Audio`] resource to play audio.
#[derive(Default)]
pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioOutput<AudioSource>>()
            .add_asset::<AudioSource>()
            .add_asset::<AudioSink>()
            .add_asset::<SpatialAudioSink>()
            .init_resource::<Audio<AudioSource>>()
            .add_system(play_queued_audio_system::<AudioSource>.in_base_set(CoreSet::PostUpdate));

        #[cfg(any(feature = "mp3", feature = "flac", feature = "wav", feature = "vorbis"))]
        app.init_asset_loader::<AudioLoader>();
    }
}

impl AddAudioSource for App {
    fn add_audio_source<T>(&mut self) -> &mut Self
    where
        T: Decodable + Asset,
        f32: rodio::cpal::FromSample<T::DecoderItem>,
    {
        self.add_asset::<T>()
            .init_resource::<Audio<T>>()
            .init_resource::<AudioOutput<T>>()
            .add_system(play_queued_audio_system::<T>.in_base_set(CoreSet::PostUpdate))
    }
}
