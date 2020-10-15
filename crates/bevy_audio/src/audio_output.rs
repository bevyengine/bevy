use crate::{AudioSource, Decodable};
use bevy_asset::{Assets, Handle};
use bevy_ecs::{Resources, World};
use parking_lot::RwLock;
use rodio::{OutputStream, OutputStreamHandle, Sink};
use std::{collections::VecDeque, fmt};

/// Used to play audio on the current "audio device"
pub struct AudioOutput<P = AudioSource>
where
    P: Decodable,
{
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    queue: RwLock<VecDeque<Handle<P>>>,
}

impl<P> fmt::Debug for AudioOutput<P>
where
    P: Decodable,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("AudioOutput")
            .field("queue", &self.queue)
            .finish()
    }
}

impl<P> Default for AudioOutput<P>
where
    P: Decodable,
{
    fn default() -> Self {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();

        Self {
            _stream: stream,
            stream_handle,
            queue: Default::default(),
        }
    }
}

impl<P> AudioOutput<P>
where
    P: Decodable + Send + Sync,
    <P as Decodable>::Decoder: rodio::Source + Send,
    <<P as Decodable>::Decoder as Iterator>::Item: rodio::Sample + Send,
{
    pub fn play_source(&self, audio_source: &P) {
        let sink = Sink::try_new(&self.stream_handle).unwrap();
        sink.append(audio_source.decoder());
        sink.detach();
    }

    pub fn play(&self, audio_source: Handle<P>) {
        self.queue.write().push_front(audio_source);
    }

    pub fn try_play_queued(&self, audio_sources: &Assets<P>) {
        let mut queue = self.queue.write();
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

/// Plays audio currently queued in the [AudioOutput] resource
pub fn play_queued_audio_system<P>(
    _world: &mut World,
    resources: &mut Resources, /*audio_sources: Res<Assets<P>>, audio_output: Res<AudioOutput<P>>*/
) where
    P: Decodable + Send + Sync,
    <P as Decodable>::Decoder: rodio::Source + Send,
    <<P as Decodable>::Decoder as Iterator>::Item: rodio::Sample + Send,
{
    let audio_output = resources.get_thread_local::<AudioOutput<P>>().unwrap();
    if let Some(audio_sources) = resources.get::<Assets<P>>() {
        audio_output.try_play_queued(&*audio_sources);
    }
}
