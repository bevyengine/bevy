use crate::AudioSource;
use bevy_asset::{Assets, Handle};
use bevy_ecs::Res;
use rodio::{Decoder, Device, Sink};
use std::{collections::VecDeque, io::Cursor, sync::RwLock};

/// Used to play audio on the current "audio device"
pub struct AudioOutput {
    device: Device,
    queue: RwLock<VecDeque<Handle<AudioSource>>>,
}

impl Default for AudioOutput {
    fn default() -> Self {
        Self {
            device: rodio::default_output_device().unwrap(),
            queue: Default::default(),
        }
    }
}

impl AudioOutput {
    pub fn play_source(&self, audio_source: &AudioSource) {
        let sink = Sink::new(&self.device);
        sink.append(Decoder::new(Cursor::new(audio_source.clone())).unwrap());
        sink.detach();
    }

    pub fn play(&self, audio_source: Handle<AudioSource>) {
        self.queue.write().unwrap().push_front(audio_source);
    }

    pub fn try_play_queued(&self, audio_sources: &Assets<AudioSource>) {
        let mut queue = self.queue.write().unwrap();
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
pub(crate) fn play_queued_audio_system(
    audio_sources: Res<Assets<AudioSource>>,
    audio_output: Res<AudioOutput>,
) {
    audio_output.try_play_queued(&audio_sources);
}
