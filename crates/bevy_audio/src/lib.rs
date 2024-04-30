#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

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
//!     commands.play_sound_with_settings("background_audio.ogg", PlaybackSettings::LOOP);
//! }
//! ```

mod audio;
mod audio_output;
mod audio_source;
mod pitch;
mod sinks;

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        AudioBundle, AudioSink, AudioSinkPlayback, AudioSource, AudioSourceBundle,
        AudioSpawnCommandExt, Decodable, GlobalVolume, Pitch, PitchBundle, PlaybackSettings,
        SpatialAudioSink, SpatialListener,
    };
}

pub use audio::*;
pub use audio_source::*;
pub use pitch::*;

pub use rodio::cpal::Sample as CpalSample;
pub use rodio::source::Source;
pub use rodio::Sample;
pub use sinks::*;

use bevy_app::prelude::*;
use bevy_asset::{Asset, AssetApp, AssetPath, AssetServer};
use bevy_ecs::{prelude::*, world::Command};
use bevy_transform::TransformSystem;

use audio_output::*;

/// Set for the audio playback systems, so they can share a run condition
#[derive(SystemSet, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
struct AudioPlaySet;

/// Adds support for audio playback to a Bevy Application
///
/// Insert an [`AudioBundle`] onto your entities to play audio.
#[derive(Default)]
pub struct AudioPlugin {
    /// The global volume for all audio entities.
    pub global_volume: GlobalVolume,
    /// The scale factor applied to the positions of audio sources and listeners for
    /// spatial audio.
    pub default_spatial_scale: SpatialScale,
}

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Volume>()
            .register_type::<GlobalVolume>()
            .register_type::<SpatialListener>()
            .register_type::<DefaultSpatialScale>()
            .register_type::<PlaybackMode>()
            .register_type::<PlaybackSettings>()
            .insert_resource(self.global_volume)
            .insert_resource(DefaultSpatialScale(self.default_spatial_scale))
            .configure_sets(
                PostUpdate,
                AudioPlaySet
                    .run_if(audio_output_available)
                    .after(TransformSystem::TransformPropagate), // For spatial audio transforms
            )
            .add_systems(
                PostUpdate,
                (update_emitter_positions, update_listener_positions).in_set(AudioPlaySet),
            )
            .init_resource::<AudioOutput>();

        #[cfg(any(feature = "mp3", feature = "flac", feature = "wav", feature = "vorbis"))]
        {
            app.add_audio_source::<AudioSource>();
            app.init_asset_loader::<AudioLoader>();
        }

        app.add_audio_source::<Pitch>();
    }
}

impl AddAudioSource for App {
    fn add_audio_source<T>(&mut self) -> &mut Self
    where
        T: Decodable + Asset,
        f32: rodio::cpal::FromSample<T::DecoderItem>,
    {
        self.init_asset::<T>().add_systems(
            PostUpdate,
            (play_queued_audio_system::<T>, cleanup_finished_audio::<T>).in_set(AudioPlaySet),
        );
        self
    }
}

/// Command for playing a standard bevy audio asset
pub struct AudioSpawnCommand<'a> {
    /// Path to the sound asset
    pub path: AssetPath<'a>,
    /// Sound playback settings
    pub settings: PlaybackSettings,
}

impl Command for AudioSpawnCommand<'static> {
    fn apply(self, world: &mut World) {
        let asset = world.get_resource::<AssetServer>().unwrap();
        let source = asset.load(&self.path);
        world.spawn(AudioBundle {
            source,
            settings: self.settings,
        });
    }
}

/// Trait for playing sounds with commands
pub trait AudioSpawnCommandExt {
    /// Command for playing a standard bevy audio asset with default settings.
    ///
    /// Remember that if the sound asset is not already loaded, the sound will have delay before playing because it needs to load first.
    fn play_sound(&mut self, data: impl Into<AssetPath<'static>>);

    /// Command for playing a standard bevy audio asset with settings.
    ///
    /// Remember that if the sound asset is not already loaded, the sound will have delay before playing because it needs to load first.
    fn play_sound_with_settings(
        &mut self,
        asset_id: impl Into<AssetPath<'static>>,
        settings: PlaybackSettings,
    );
}

impl<'w, 's> AudioSpawnCommandExt for Commands<'w, 's> {
    fn play_sound(&mut self, path: impl Into<AssetPath<'static>>) {
        self.add(AudioSpawnCommand {
            path: path.into(),
            settings: Default::default(),
        });
    }
    fn play_sound_with_settings(
        &mut self,
        path: impl Into<AssetPath<'static>>,
        settings: PlaybackSettings,
    ) {
        self.add(AudioSpawnCommand {
            path: path.into().clone(),
            settings,
        });
    }
}
