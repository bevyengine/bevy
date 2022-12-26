use crate::{AudioSink, AudioSource, Decodable};
use bevy_asset::{Asset, Handle, HandleId};
use bevy_ecs::system::Resource;
use parking_lot::RwLock;
use std::{collections::VecDeque, fmt};

/// A resource for playing audio.
#[derive(Resource)]
pub struct Audio<Source = AudioSource>
where
    Source: Asset + Decodable,
{
    /// A queue of audio to be played, stored in a lockable VecDeque.
    pub(crate) playback_queue: RwLock<VecDeque<AudioToPlay<Source>>>,
}

impl<Source: Asset> fmt::Debug for Audio<Source>
where
    Source: Decodable,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Audio")
            .field("playback_queue", &self.playback_queue)
            .finish()
    }
}

impl<Source> Default for Audio<Source>
where
    Source: Asset + Decodable,
{
    fn default() -> Self {
        Self {
            playback_queue: Default::default(),
        }
    }
}

impl<Source> Audio<Source>
where
    Source: Asset + Decodable,
{
    /// Play audio from a handle to the audio source.
    ///
    /// Returns a weak handle to the `AudioSink`. If this handle isn't changed to a
    /// strong one, the sink will be detached and the sound will continue playing. Changing it
    /// to a strong handle allows for control on the playback through the `AudioSink` asset.
    pub fn play(&self, audio_source: Handle<Source>) -> Handle<AudioSink> {
        let id = HandleId::random::<AudioSink>();
        let audio_to_play = AudioToPlay {
            settings: PlaybackSettings::ONCE,
            sink_handle: id,
            source_handle: audio_source,
        };
        self.playback_queue.write().push_back(audio_to_play);
        Handle::<AudioSink>::weak(id)
    }

    /// Play audio from a handle to the audio source with `PlaybackSettings` that
    /// allows looping or changing volume from the start.
    ///
    /// Returns a weak handle to the `AudioSink`. If this handle isn't changed to a
    /// strong one, the sink will be detached and the sound will continue playing. Changing it
    /// to a strong handle allows for control on the playback through the `AudioSink` asset.
    pub fn play_with_settings(
        &self,
        audio_source: Handle<Source>,
        settings: PlaybackSettings,
    ) -> Handle<AudioSink> {
        let id = HandleId::random::<AudioSink>();
        let audio_to_play = AudioToPlay {
            settings,
            sink_handle: id,
            source_handle: audio_source,
        };
        self.playback_queue.write().push_back(audio
