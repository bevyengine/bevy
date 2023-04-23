use crate::{AudioSink, AudioSource, Decodable, SpatialAudioSink};
use bevy_asset::{Asset, AssetHandleProvider, Assets, Handle, UntypedAssetId};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    system::Resource,
    world::{FromWorld, World},
};
use bevy_math::Vec3;
use bevy_transform::prelude::Transform;
use parking_lot::RwLock;
use std::{collections::VecDeque, fmt};

/// Use this [`Resource`] to play audio.
///
/// ```
/// # use bevy_ecs::system::Res;
/// # use bevy_asset::AssetServer;
/// # use bevy_audio::Audio;
/// fn play_audio_system(asset_server: Res<AssetServer>, audio: Res<Audio>) {
///     audio.play(asset_server.load("my_sound.ogg"));
/// }
/// ```
#[derive(Resource)]
pub struct Audio<Source = AudioSource>
where
    Source: Asset + Decodable,
{
    sink_handle_provider: AssetHandleProvider,
    spatial_sink_handle_provider: AssetHandleProvider,
    /// Queue for playing audio from asset handles
    pub(crate) queue: RwLock<VecDeque<AudioToPlay<Source>>>,
}

impl<Source: Asset> fmt::Debug for Audio<Source>
where
    Source: Decodable,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Audio").field("queue", &self.queue).finish()
    }
}

impl<Source> FromWorld for Audio<Source>
where
    Source: Asset + Decodable,
{
    fn from_world(world: &mut World) -> Self {
        Self {
            sink_handle_provider: world.resource::<Assets<AudioSink>>().get_handle_provider(),
            spatial_sink_handle_provider: world
                .resource::<Assets<SpatialAudioSink>>()
                .get_handle_provider(),
            queue: Default::default(),
        }
    }
}

impl<Source> Audio<Source>
where
    Source: Asset + Decodable,
{
    /// Play audio from a [`Handle`] to the audio source
    ///
    /// ```
    /// # use bevy_ecs::system::Res;
    /// # use bevy_asset::AssetServer;
    /// # use bevy_audio::Audio;
    /// fn play_audio_system(asset_server: Res<AssetServer>, audio: Res<Audio>) {
    ///     audio.play(asset_server.load("my_sound.ogg"));
    /// }
    /// ```
    ///
    /// Returns a strong [`Handle`] to the [`AudioSink`]. The strong handle allows you to control the playback through the [`AudioSink`] asset.
    ///
    /// ```
    /// # use bevy_ecs::system::Res;
    /// # use bevy_asset::{AssetServer, Assets};
    /// # use bevy_audio::{Audio, AudioSink};
    /// fn play_audio_system(
    ///     asset_server: Res<AssetServer>,
    ///     audio: Res<Audio>,
    ///     audio_sinks: Res<Assets<AudioSink>>,
    /// ) {
    ///     // This is a weak handle, and can't be used to control playback.
    ///     let weak_handle = audio.play(asset_server.load("my_sound.ogg"));
    ///     // This is now a strong handle, and can be used to control playback.
    ///     let strong_handle = audio_sinks.get_handle(weak_handle);
    /// }
    /// ```
    pub fn play(&self, audio_source: Handle<Source>) -> Handle<AudioSink> {
        let handle = self
            .sink_handle_provider
            .reserve_handle()
            .typed_debug_checked();
        let config = AudioToPlay {
            settings: PlaybackSettings::ONCE,
            sink_asset: handle.id().untyped(),
            source_handle: audio_source,
            spatial: None,
        };
        self.queue.write().push_back(config);
        handle
    }

    /// Play audio from a [`Handle`] to the audio source with [`PlaybackSettings`] that
    /// allows looping or changing volume from the start.
    ///
    /// ```
    /// # use bevy_ecs::system::Res;
    /// # use bevy_asset::AssetServer;
    /// # use bevy_audio::{Audio, Volume};
    /// # use bevy_audio::PlaybackSettings;
    /// fn play_audio_system(asset_server: Res<AssetServer>, audio: Res<Audio>) {
    ///     audio.play_with_settings(
    ///         asset_server.load("my_sound.ogg"),
    ///         PlaybackSettings::LOOP.with_volume(Volume::new_relative(0.75)),
    ///     );
    /// }
    /// ```
    ///
    /// See [`Self::play`] on how to control playback once it's started.
    pub fn play_with_settings(
        &self,
        audio_source: Handle<Source>,
        settings: PlaybackSettings,
    ) -> Handle<AudioSink> {
        let handle = self
            .sink_handle_provider
            .reserve_handle()
            .typed_debug_checked();
        let config = AudioToPlay {
            settings,
            sink_asset: handle.id().untyped(),
            source_handle: audio_source,
            spatial: None,
        };
        self.queue.write().push_back(config);
        handle
    }

    /// Play audio from a [`Handle`] to the audio source, placing the listener at the given
    /// transform, an ear on each side separated by `gap`. The audio emitter will placed at
    /// `emitter`.
    ///
    /// `bevy_audio` is not using HRTF for spatial audio, but is transforming the sound to a mono
    /// track, and then changing the level of each stereo channel according to the distance between
    /// the emitter and each ear by amplifying the difference between what the two ears hear.
    ///
    /// ```
    /// # use bevy_ecs::system::Res;
    /// # use bevy_asset::AssetServer;
    /// # use bevy_audio::Audio;
    /// # use bevy_math::Vec3;
    /// # use bevy_transform::prelude::Transform;
    /// fn play_spatial_audio_system(asset_server: Res<AssetServer>, audio: Res<Audio>) {
    ///     // Sound will be to the left and behind the listener
    ///     audio.play_spatial(
    ///         asset_server.load("my_sound.ogg"),
    ///         Transform::IDENTITY,
    ///         1.0,
    ///         Vec3::new(-2.0, 0.0, 1.0),
    ///     );
    /// }
    /// ```
    ///
    /// Returns a [`Handle`] to the [`SpatialAudioSink`], which allows you to control the playback, or move the listener and emitter
    /// through the [`SpatialAudioSink`] asset.
    ///
    /// ```
    /// # use bevy_ecs::system::Res;
    /// # use bevy_asset::{AssetServer, Assets};
    /// # use bevy_audio::{Audio, SpatialAudioSink};
    /// # use bevy_math::Vec3;
    /// # use bevy_transform::prelude::Transform;
    /// fn play_spatial_audio_system(
    ///     asset_server: Res<AssetServer>,
    ///     audio: Res<Audio>,
    ///     spatial_audio_sinks: Res<Assets<SpatialAudioSink>>,
    /// ) {
    ///     // This is a weak handle, and can't be used to control playback.
    ///     let weak_handle = audio.play_spatial(
    ///         asset_server.load("my_sound.ogg"),
    ///         Transform::IDENTITY,
    ///         1.0,
    ///         Vec3::new(-2.0, 0.0, 1.0),
    ///     );
    ///     // This is now a strong handle, and can be used to control playback, or move the emitter.
    ///     let strong_handle = spatial_audio_sinks.get_handle(weak_handle);
    /// }
    /// ```
    pub fn play_spatial(
        &self,
        audio_source: Handle<Source>,
        listener: Transform,
        gap: f32,
        emitter: Vec3,
    ) -> Handle<SpatialAudioSink> {
        let handle = self
            .spatial_sink_handle_provider
            .reserve_handle()
            .typed_debug_checked();
        let config = AudioToPlay {
            settings: PlaybackSettings::ONCE,
            sink_asset: handle.id().untyped(),
            source_handle: audio_source,
            spatial: Some(SpatialSettings {
                left_ear: (listener.translation + listener.left() * gap / 2.0).to_array(),
                right_ear: (listener.translation + listener.right() * gap / 2.0).to_array(),
                emitter: emitter.to_array(),
            }),
        };
        self.queue.write().push_back(config);
        handle
    }

    /// Play spatial audio from a [`Handle`] to the audio source with [`PlaybackSettings`] that
    /// allows looping or changing volume from the start. The listener is placed at the given
    /// transform, an ear on each side separated by `gap`. The audio emitter is placed at
    /// `emitter`.
    ///
    /// `bevy_audio` is not using HRTF for spatial audio, but is transforming the sound to a mono
    /// track, and then changing the level of each stereo channel according to the distance between
    /// the emitter and each ear by amplifying the difference between what the two ears hear.
    ///
    /// ```
    /// # use bevy_ecs::system::Res;
    /// # use bevy_asset::AssetServer;
    /// # use bevy_audio::{Audio, Volume};
    /// # use bevy_audio::PlaybackSettings;
    /// # use bevy_math::Vec3;
    /// # use bevy_transform::prelude::Transform;
    /// fn play_spatial_audio_system(asset_server: Res<AssetServer>, audio: Res<Audio>) {
    ///     audio.play_spatial_with_settings(
    ///         asset_server.load("my_sound.ogg"),
    ///         PlaybackSettings::LOOP.with_volume(Volume::new_relative(0.75)),
    ///         Transform::IDENTITY,
    ///         1.0,
    ///         Vec3::new(-2.0, 0.0, 1.0),
    ///     );
    /// }
    /// ```
    ///
    /// See [`Self::play_spatial`] on how to control playback once it's started, or how to move
    /// the listener or the emitter.
    pub fn play_spatial_with_settings(
        &self,
        audio_source: Handle<Source>,
        settings: PlaybackSettings,
        listener: Transform,
        gap: f32,
        emitter: Vec3,
    ) -> Handle<SpatialAudioSink> {
        let handle = self
            .spatial_sink_handle_provider
            .reserve_handle()
            .typed_debug_checked();
        let config = AudioToPlay {
            settings,
            sink_asset: handle.id().untyped(),
            source_handle: audio_source,
            spatial: Some(SpatialSettings {
                left_ear: (listener.translation + listener.left() * gap / 2.0).to_array(),
                right_ear: (listener.translation + listener.right() * gap / 2.0).to_array(),
                emitter: emitter.to_array(),
            }),
        };
        self.queue.write().push_back(config);
        handle
    }
}

/// Defines the volume to play an audio source at.
#[derive(Clone, Copy, Debug)]
pub enum Volume {
    /// A volume level relative to the global volume.
    Relative(VolumeLevel),
    /// A volume level that ignores the global volume.
    Absolute(VolumeLevel),
}

impl Default for Volume {
    fn default() -> Self {
        Self::Relative(VolumeLevel::default())
    }
}

impl Volume {
    /// Create a new volume level relative to the global volume.
    pub fn new_relative(volume: f32) -> Self {
        Self::Relative(VolumeLevel::new(volume))
    }
    /// Create a new volume level that ignores the global volume.
    pub fn new_absolute(volume: f32) -> Self {
        Self::Absolute(VolumeLevel::new(volume))
    }
}

/// A volume level equivalent to a positive only float.
#[derive(Clone, Copy, Deref, DerefMut, Debug)]
pub struct VolumeLevel(pub(crate) f32);

impl Default for VolumeLevel {
    fn default() -> Self {
        Self(1.0)
    }
}

impl VolumeLevel {
    /// Create a new volume level.
    pub fn new(volume: f32) -> Self {
        debug_assert!(volume >= 0.0);
        Self(volume)
    }
    /// Get the value of the volume level.
    pub fn get(&self) -> f32 {
        self.0
    }
}

/// Settings to control playback from the start.
#[derive(Clone, Copy, Debug)]
pub struct PlaybackSettings {
    /// Play in repeat
    pub repeat: bool,
    /// Volume to play at.
    pub volume: Volume,
    /// Speed to play at.
    pub speed: f32,
}

impl Default for PlaybackSettings {
    fn default() -> Self {
        Self::ONCE
    }
}

impl PlaybackSettings {
    /// Will play the associate audio source once.
    pub const ONCE: PlaybackSettings = PlaybackSettings {
        repeat: false,
        volume: Volume::Relative(VolumeLevel(1.0)),
        speed: 1.0,
    };

    /// Will play the associate audio source in a loop.
    pub const LOOP: PlaybackSettings = PlaybackSettings {
        repeat: true,
        volume: Volume::Relative(VolumeLevel(1.0)),
        speed: 1.0,
    };

    /// Helper to set the volume from start of playback.
    pub const fn with_volume(mut self, volume: Volume) -> Self {
        self.volume = volume;
        self
    }

    /// Helper to set the speed from start of playback.
    pub const fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
}

#[derive(Clone)]
pub(crate) struct SpatialSettings {
    pub(crate) left_ear: [f32; 3],
    pub(crate) right_ear: [f32; 3],
    pub(crate) emitter: [f32; 3],
}

#[derive(Clone)]
pub(crate) struct AudioToPlay<Source>
where
    Source: Asset + Decodable,
{
    pub(crate) sink_asset: UntypedAssetId,
    pub(crate) source_handle: Handle<Source>,
    pub(crate) settings: PlaybackSettings,
    pub(crate) spatial: Option<SpatialSettings>,
}

impl<Source> fmt::Debug for AudioToPlay<Source>
where
    Source: Asset + Decodable,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AudioToPlay")
            .field("sink_asset", &self.sink_asset)
            .field("source_handle", &self.source_handle)
            .field("settings", &self.settings)
            .finish()
    }
}

/// Use this [`Resource`] to control the global volume of all audio with a [`Volume::Relative`] volume.
///
/// Keep in mind that changing this value will not affect already playing audio.
#[derive(Resource, Default, Clone, Copy)]
pub struct GlobalVolume {
    /// The global volume of all audio.
    pub volume: VolumeLevel,
}

impl GlobalVolume {
    /// Create a new [`GlobalVolume`] with the given volume.
    pub fn new(volume: f32) -> Self {
        Self {
            volume: VolumeLevel::new(volume),
        }
    }
}
