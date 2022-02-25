use crate::{Audio, AudioSource, Decodable};
use bevy_asset::{Asset, Assets};
use bevy_ecs::world::World;
use bevy_utils::tracing::warn;
use rodio::{OutputStream, OutputStreamHandle, Sink};
use std::marker::PhantomData;

/// Used internally to play audio on the current "audio device"
pub struct AudioOutput<Source = AudioSource>
where
    Source: Decodable,
{
    _stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    phantom: PhantomData<Source>,
}

impl<Source> Default for AudioOutput<Source>
where
    Source: Decodable,
{
    fn default() -> Self {
        if let Ok((stream, stream_handle)) = OutputStream::try_default() {
            Self {
                _stream: Some(stream),
                stream_handle: Some(stream_handle),
                phantom: PhantomData,
            }
        } else {
            warn!("No audio device found.");
            Self {
                _stream: None,
                stream_handle: None,
                phantom: PhantomData,
            }
        }
    }
}

impl<Source> AudioOutput<Source>
where
    Source: Asset + Decodable,
{
    fn play_source(&self, audio_source: &Source) {
        if let Some(stream_handle) = &self.stream_handle {
            let sink = Sink::try_new(stream_handle).unwrap();
            sink.append(audio_source.decoder());
            sink.detach();
        }
    }

    fn try_play_queued(&self, audio_sources: &Assets<Source>, audio: &mut Audio<Source>) {
        let mut queue = audio.queue.write();
        let len = queue.len();
        let mut i = 0;
        while i < len {
            let audio_source_handle = queue.pop_back().unwrap();
            if let Some(audio_source) = audio_sources.get(&audio_source_handle) {
                self.play_source(audio_source);
            } else {
                // audio source hasn't loaded yet. add it back to the queue
                queue.push_front(audio_source_handle);
            }
            i += 1;
        }
    }
}

/// Plays audio currently queued in the [`Audio`] resource through the [`AudioOutput`] resource
pub fn play_queued_audio_system<Source: Asset>(world: &mut World)
where
    Source: Decodable,
{
    let world = world.cell();
    let audio_output = world.get_non_send::<AudioOutput<Source>>().unwrap();
    let mut audio = world.get_resource_mut::<Audio<Source>>().unwrap();

    if let Some(audio_sources) = world.get_resource::<Assets<Source>>() {
        audio_output.try_play_queued(&*audio_sources, &mut *audio);
    };
}
