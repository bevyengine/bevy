use crate::{AudioSink, AudioSource, Decodable};
use bevy_asset::{Asset, Handle, HandleId};
use bevy_ecs::system::Resource;
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

impl<Source> Default for Audio<Source>
where
    Source: Asset + Decodable,
{
    fn default() -> Self {
        Self {
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
    /// Returns a weak [`Handle`] to the [`AudioSink`]. If this handle isn't changed to a
    /// strong one, the sink will be detached and the sound will continue playing. Changing it
    /// to a strong handle allows for control on the playback through the [`AudioSink`] asset.
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
        let id = HandleId::random::<AudioSink>();
        let config = AudioToPlay {
            settings: PlaybackSettings::ONCE,
            sink_handle: id,
            source_handle: audio_source,
        };
        self.queue.write().push_back(config);
        Handle::<AudioSink>::weak(id)
    }

    /// Play audio from a [`Handle`] to the audio source with [`PlaybackSettings`] that
    /// allows looping or changing volume from the start.
    ///
    /// ```
    /// # use bevy_ecs::system::Res;
    /// # use bevy_asset::AssetServer;
    /// # use bevy_audio::Audio;
    /// # use bevy_audio::PlaybackSettings;
    /// fn play_audio_system(asset_server: Res<AssetServer>, audio: Res<Audio>) {
    ///     audio.play_with_settings(
    ///         asset_server.load("my_sound.ogg"),
    ///         PlaybackSettings::LOOP.with_volume(0.75),
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
        let id = HandleId::random::<AudioSink>();
        let config = AudioToPlay {
            settings,
            sink_handle: id,
            source_handle: audio_source,
        };
        self.queue.write().push_back(config);
        Handle::<AudioSink>::weak(id)
    }
}

/// Settings to control playback from the start.
#[derive(Clone, Debug)]
pub struct PlaybackSettings {
    /// Play in repeat
    pub repeat: bool,
    /// Volume to play at.
    pub volume: f32,
    /// Ignore global volume
    pub absolute_volume: bool,
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
        volume: 1.0,
        absolute_volume: false,
        speed: 1.0,
    };

    /// Will play the associate audio source in a loop.
    pub const LOOP: PlaybackSettings = PlaybackSettings {
        repeat: true,
        volume: 1.0,
        absolute_volume: false,
        speed: 1.0,
    };

    /// Helper to set the volume from start of playback.
    pub const fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume;
        self
    }

    /// Helper to set the speed from start of playback.
    pub const fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    /// Helper to set if the volume should ignore the global volume.
    pub const fn with_absolute_volume(mut self, absolute_volume: bool) -> Self {
        self.absolute_volume = absolute_volume;
        self
    }
}

#[derive(Clone)]
pub(crate) struct AudioToPlay<Source>
where
    Source: Asset + Decodable,
{
    pub(crate) sink_handle: HandleId,
    pub(crate) source_handle: Handle<Source>,
    pub(crate) settings: PlaybackSettings,
}

impl<Source> fmt::Debug for AudioToPlay<Source>
where
    Source: Asset + Decodable,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AudioToPlay")
            .field("sink_handle", &self.sink_handle)
            .field("source_handle", &self.source_handle)
            .field("settings", &self.settings)
            .finish()
    }
}

/// Use this [`Resource`] to control the global volume of all non-absolute audio.
/// Non-abosolute being any audio source that doesn't have [`absolute_volume`](PlaybackSettings::absolute_volume) set to `true`.
#[derive(Resource)]
pub struct GlobalVolume {
    /// The global volume of all audio.
    pub volume: f32,
}

impl Default for GlobalVolume {
    fn default() -> Self {
        Self { volume: 1.0 }
    }
}

impl GlobalVolume {
    /// Create a new [`GlobalVolume`] with the given volume.
    pub const fn new(volume: f32) -> Self {
        Self { volume }
    }
}
