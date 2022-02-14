use crate::{AudioSink, AudioSource, Decodable};
use bevy_asset::{Asset, Handle, HandleId};
use parking_lot::RwLock;
use std::{collections::VecDeque, fmt};

/// Use this resource to play audio
///
/// ```
/// # use bevy_ecs::system::Res;
/// # use bevy_asset::AssetServer;
/// # use bevy_audio::Audio;
/// fn play_audio_system(asset_server: Res<AssetServer>, audio: Res<Audio>) {
///     audio.play(asset_server.load("my_sound.ogg"));
/// }
/// ```
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
            repeat: false,
            sink_handle: id,
            source_handle: audio_source,
        };
        self.queue.write().push_back(config);
        Handle::<AudioSink>::weak(id)
    }

    /// Play audio from a [`Handle`] to the audio source in a loop
    ///
    /// See [`Self::play`] on how to control playback.
    pub fn play_in_loop(&self, audio_source: Handle<Source>) -> Handle<AudioSink> {
        let id = HandleId::random::<AudioSink>();
        let config = AudioToPlay {
            repeat: true,
            sink_handle: id,
            source_handle: audio_source,
        };
        self.queue.write().push_back(config);
        Handle::<AudioSink>::weak(id)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct AudioToPlay<Source>
where
    Source: Asset + Decodable,
{
    pub(crate) sink_handle: HandleId,
    pub(crate) source_handle: Handle<Source>,
    pub(crate) repeat: bool,
}

impl<Source> fmt::Debug for AudioToPlay<Source>
where
    Source: Asset + Decodable,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AudioToPlay")
            .field("sink_handle", &self.sink_handle)
            .field("source_handle", &self.source_handle)
            .field("repeat", &self.repeat)
            .finish()
    }
}
